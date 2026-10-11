#include "taglib_bridge.h"
#include <tiostream.h>
#include <mpegfile.h>
#include <flacfile.h>
#include <wavfile.h>
#include <aifffile.h>
#include <dsffile.h>
#include <wavpackfile.h>
#include <vorbisfile.h>
#include <opusfile.h>
#include <mp4file.h>
#include <mp4tag.h>
#include <mp4item.h>
#include <id3v2tag.h>
#include <textidentificationframe.h>
#include <xiphcomment.h>
#include <apetag.h>
#include <apeitem.h>
#include <algorithm>
#include <cstring>
#include <limits>
#include <memory>
#include <map>
#include <stdexcept>
#include <vector>

namespace {
constexpr size_t max_block = 16 * 1024 * 1024;
struct InputFailure {};
struct Unsupported {};
class ReadStream final : public TagLib::IOStream {
  SmInput input;
  uint64_t pos = 0, consumed = 0;
  struct Frozen { uint64_t offset; TagLib::ByteVector bytes; };
  std::vector<Frozen> frozen;
  size_t frozen_bytes = 0;
public:
  void remember(uint64_t offset, const TagLib::ByteVector& bytes) {
    if(bytes.isEmpty())return;
    constexpr size_t snapshot_budget = 32 * 1024 * 1024 + 64 * 1024;
    if(frozen.size()>=8196 || bytes.size()>snapshot_budget-frozen_bytes)throw InputFailure{};
    frozen_bytes+=bytes.size();frozen.push_back({offset,bytes});
  }
  explicit ReadStream(SmInput in): input(in) {}
  TagLib::FileName name() const override { return "readonly"; }
  TagLib::ByteVector readBlock(size_t n) override {
    n = static_cast<size_t>(std::min<uint64_t>(n, pos < input.length ? input.length - pos : 0));
    if(n > max_block || consumed > 256ULL * 1024 * 1024 - n) throw InputFailure{};
    consumed += n;
    TagLib::ByteVector bytes(n, 0);
    size_t done = 0;
    while(done<n) {
      const Frozen* hit=nullptr;size_t amount=n-done;
      for(const auto& region:frozen) {
        const uint64_t end=region.offset+region.bytes.size();
        if(pos>=region.offset && pos<end) {
          hit=&region;amount=static_cast<size_t>(std::min<uint64_t>(amount,end-pos));break;
        }
        if(region.offset>pos)amount=static_cast<size_t>(std::min<uint64_t>(amount,region.offset-pos));
      }
      if(hit) {
        std::memcpy(bytes.data()+done,hit->bytes.data()+(pos-hit->offset),amount);
      } else {
        size_t got=0;
        if(input.read_at(input.context,pos,reinterpret_cast<uint8_t*>(bytes.data()+done),amount,&got)||got!=amount)throw InputFailure{};
      }
      pos+=amount;done+=amount;
    }
    return bytes;
  }
  void writeBlock(const TagLib::ByteVector&) override { throw InputFailure{}; }
  void insert(const TagLib::ByteVector&, TagLib::offset_t, size_t) override { throw InputFailure{}; }
  void removeBlock(TagLib::offset_t, size_t) override { throw InputFailure{}; }
  void truncate(TagLib::offset_t) override { throw InputFailure{}; }
  bool readOnly() const override { return true; }
  bool isOpen() const override { return true; }
  void seek(TagLib::offset_t off, Position p = Beginning) override {
    const uint64_t base = p == Beginning ? 0 : p == Current ? pos : input.length;
    // Unsigned arithmetic after explicit underflow/overflow checks avoids signed UB.
    if(off < 0) {
      const uint64_t amount = static_cast<uint64_t>(-(off + 1)) + 1;
      if(amount > base) return; // Match FileStream failed negative seek: leave cursor unchanged.
      pos = base - amount;
    } else {
      if(static_cast<uint64_t>(off) > static_cast<uint64_t>(std::numeric_limits<int64_t>::max()) - base) throw InputFailure{};
      pos = base + static_cast<uint64_t>(off);
    }
  }
  TagLib::offset_t tell() const override { return static_cast<TagLib::offset_t>(pos); }
  TagLib::offset_t length() override { return static_cast<TagLib::offset_t>(input.length); }
};
void u32(std::vector<uint8_t>& v, uint32_t n) { for(int i=0;i<4;++i) v.push_back(static_cast<uint8_t>(n >> (8*i))); }
void add(std::vector<uint8_t>& v, uint32_t type, const std::string& s) {
 if(s.size()>max_block || v.size()>max_block-s.size() || v.size()+s.size()>max_block-8) throw InputFailure{};
 u32(v,type);u32(v,static_cast<uint32_t>(s.size()));v.insert(v.end(),s.begin(),s.end());
}
}
struct SmHandle {
 // file is destroyed before stream; the Rust context outlives both.
 ReadStream stream;
 std::unique_ptr<TagLib::File> file;
 TagLib::ID3v2::Tag* id3 = nullptr;
 TagLib::Ogg::XiphComment* xiph = nullptr;
 TagLib::APE::Tag* ape = nullptr;
 TagLib::MP4::Tag* mp4 = nullptr;
 uint32_t family = 0;
 std::map<std::string, TagLib::ByteVector> raw_id3;
 explicit SmHandle(SmInput input):stream(input){}
};

namespace {
uint32_t big32(const TagLib::ByteVector& b, size_t at) {
 uint32_t n=0;for(size_t i=0;i<4;++i)n=(n<<8)|static_cast<uint8_t>(b[at+i]);return n;
}
uint32_t sync32(const TagLib::ByteVector& b, size_t at) {
 uint32_t n=0;for(size_t i=0;i<4;++i){auto c=static_cast<uint8_t>(b[at+i]);if(c&128)throw Unsupported{};n=(n<<7)|c;}return n;
}
TagLib::ByteVector read_at(ReadStream& s, uint64_t offset, size_t n) {
 if(offset>static_cast<uint64_t>(s.length()))throw Unsupported{};
 s.seek(static_cast<int64_t>(offset));auto bytes=s.readBlock(n);
 // Bind subsequent native parsing to exactly the bytes accepted by preflight;
 // a mutable source cannot swap in over-budget metadata after validation.
 s.remember(offset,bytes);return bytes;
}
// This is a bounded raw-byte preflight, before any TagLib::File constructor.
// It allocates one capped tag body, never a per-component native string list.
void capture_id3(SmHandle& h, const TagLib::ByteVector& magic) {
 if(h.family!=1)return;
 uint64_t offset=0;bool present=false;
 if(magic.startsWith("ID3")){present=true;}
 else if(magic.startsWith("DSD ")) {
  if(magic.size()<28)throw Unsupported{};
  for(int i=7;i>=0;--i)offset=(offset<<8)|static_cast<uint8_t>(magic[20+i]);
  present=offset!=0;
 } else if(magic.startsWith("RIFF")||magic.startsWith("FORM")) {
  bool little=magic.startsWith("RIFF");uint64_t cursor=12;size_t chunks=0, extra_metadata=0, info_fields=0;
  while(cursor+8<=static_cast<uint64_t>(h.stream.length())) {
   if(++chunks>4096)throw InputFailure{};
   auto header=read_at(h.stream,cursor,8);uint32_t length=big32(header,4);
   if(little)length=(length>>24)|((length>>8)&0xff00)|((length<<8)&0xff0000)|(length<<24);
   if(length>static_cast<uint64_t>(h.stream.length())-cursor-8)throw Unsupported{};
   if(header.startsWith("ID3 ")||header.startsWith("id3 ")) {
    if(present)throw Unsupported{};
    offset=cursor+8;present=true;
   }
   // The legacy projection ignores INFO, but TagLib still parses it. Bound
   // its field count and all extra metadata bytes before the WAV constructor.
   if(little && (header.startsWith("LIST")||header.startsWith("iXML")||header.startsWith("bext"))) {
    if(length>max_block-extra_metadata)throw InputFailure{};
    extra_metadata+=length;
    if(header.startsWith("LIST")) {
     const auto list=read_at(h.stream,cursor+8,length);
     if(list.startsWith("INFO")) {
      size_t field=4;
      while(field<list.size()) {
       if(++info_fields>4096)throw InputFailure{};
       if(list.size()-field<8)throw Unsupported{};
       uint32_t size=big32(list,field+4);
       size=(size>>24)|((size>>8)&0xff00)|((size<<8)&0xff0000)|(size<<24);
       if(size>list.size()-field-8)throw Unsupported{};
       field+=8ULL+size+(size&1);
      }
     }
    }
   }
   cursor+=8ULL+length+(length&1);
  }
 }
 if(!present)return;
 auto head=read_at(h.stream,offset,10);
 if(head.size()!=10 || !head.startsWith("ID3"))throw Unsupported{};
 const auto version=static_cast<uint8_t>(head[3]);
 if((version!=3&&version!=4)||head[5]!=0)throw Unsupported{}; // v2.2, unsync, extension/footer not proven
 const auto size=sync32(head,6);
 if(size>max_block || offset+10ULL+size>static_cast<uint64_t>(h.stream.length()))throw Unsupported{};
 auto body=read_at(h.stream,offset+10,size);
 size_t at=0, frames=0, components=0;
 while(at<body.size() && body[at]!=0) {
  if(++frames>4096)throw InputFailure{};
  if(body.size()-at<10)throw Unsupported{};
  std::string key(body.data()+at,4);
  const uint32_t n=version==4?sync32(body,at+4):big32(body,at+4);
  if(n>body.size()-at-10)throw Unsupported{};
  // All frames, including fields ignored by the compatibility projection,
  // are parsed by TagLib. Reject flags before decompression/decryption and
  // reject structured/embedded frame classes lacking bounded preflight here.
  if(body[at+8]!=0||body[at+9]!=0)throw Unsupported{};
  // Pinned FrameFactory upgrades IPLS to TIPL in v2.3. Its iTunes
  // three-character compatibility IDs have a separate v2.2 conversion table;
  // reject that representation rather than admitting unclassified aliases.
  if(version==3 && (key[3]=='\0'||key[3]==' '))throw Unsupported{};
  const bool text=(version==3 && key=="IPLS")||key[0]=='T'||key=="WFED"||key=="MVNM"||key=="MVIN"||key=="GRP1";
  if(key=="CHAP"||key=="CTOC"||key=="SYLT"||key=="ETCO"||key=="RVA2")throw Unsupported{};
  if(text) {
   if(n<1)throw Unsupported{};
   const auto encoding=static_cast<uint8_t>(body[at+10]);
   if(encoding>3)throw Unsupported{};
   const size_t width=(encoding==1||encoding==2)?2:1;
   const size_t start=at+11, end=at+10+n;
   if((end-start)%width)throw Unsupported{};
   size_t count=0;
   for(size_t cursor=start;cursor<end;cursor+=width) {
    const bool delimiter=body[cursor]==0&&(width==1||body[cursor+1]==0);
    if(delimiter && ++count>4096)throw InputFailure{};
    // TagLib's genre upgrader can expand parenthesized legacy genre syntax
    // into additional strings. Charge those tokens conservatively as well.
    if(key=="TCON" && (body[cursor]=='(' || (width==2 && body[cursor+1]=='(')) && ++components>4096)throw InputFailure{};
   }
   const bool terminated=end>start && body[end-width]==0&&(width==1||body[end-1]==0);
   if(!terminated)++count;
   if(count>4096-components)throw InputFailure{};
   components+=count;
  }
  if(key=="TPE1"||key=="TPE2"||key=="TIT2"||key=="TALB"||key=="TCON"||key=="TRCK") {
   if(h.raw_id3.count(key))throw Unsupported{};
   h.raw_id3.emplace(key,body.mid(at+10,n));
  }
  at+=10+n;
 }
}
}
extern "C" int32_t sm_open(const SmInput* input, SmHandle** out) {
 if(!out) return 1;
 *out=nullptr;
 if(!input || input->version!=1 || input->struct_size!=sizeof(SmInput) || !input->read_at || !input->context || input->length>static_cast<uint64_t>(std::numeric_limits<int64_t>::max())) return 1;
 try {
  auto h=std::make_unique<SmHandle>(*input);
  const auto magic=h->stream.readBlock(64);h->stream.remember(0,magic);h->stream.seek(0);
  const auto prefix=[&](const char*s, size_t n, size_t at=0){ return magic.size()>=at+n && std::memcmp(magic.data()+at,s,n)==0; };
  if(prefix("0\x26\xb2\x75\x8e\x66\xcf\x11",8)) return 3; // ASF legacy unsupported
  const bool id3_container=prefix("ID3",3)||prefix("DSD ",4)||
   (prefix("RIFF",4)&&prefix("WAVE",4,8))||
   (prefix("FORM",4)&&(prefix("AIFF",4,8)||prefix("AIFC",4,8)))||
   (magic.size()>=2 && static_cast<uint8_t>(magic[0])==255 && (static_cast<uint8_t>(magic[1])&0xe6)==0xe2);
  if(id3_container) {
   h->family=1;
   capture_id3(*h,magic);
   h->stream.seek(0);
  }
  if(prefix("ftyp",4,4)) {
   auto f=std::make_unique<TagLib::MP4::File>(&h->stream,false); h->mp4=f->tag();h->file=std::move(f);h->family=4;
  } else if(prefix("fLaC",4)) {
   auto f=std::make_unique<TagLib::FLAC::File>(&h->stream,false);h->xiph=f->xiphComment();h->file=std::move(f);h->family=2;
  } else if(prefix("RIFF",4)&&prefix("WAVE",4,8)) {
   auto f=std::make_unique<TagLib::RIFF::WAV::File>(&h->stream,false);h->id3=f->ID3v2Tag();h->file=std::move(f);h->family=1;
  } else if(prefix("FORM",4)&&(prefix("AIFF",4,8)||prefix("AIFC",4,8))) {
   auto f=std::make_unique<TagLib::RIFF::AIFF::File>(&h->stream,false);h->id3=f->tag();h->file=std::move(f);h->family=1;
  } else if(prefix("DSD ",4)) {
   auto f=std::make_unique<TagLib::DSF::File>(&h->stream,false);h->id3=f->tag();h->file=std::move(f);h->family=1;
  } else if(prefix("wvpk",4)) {
   auto f=std::make_unique<TagLib::WavPack::File>(&h->stream,false);h->ape=f->APETag();h->file=std::move(f);h->family=3;
  } else if(prefix("OggS",4)) {
   if(magic.find("OpusHead") >= 0) { auto f=std::make_unique<TagLib::Ogg::Opus::File>(&h->stream,false);h->xiph=f->tag();h->file=std::move(f); }
   else if(magic.find("vorbis") >= 0) { auto f=std::make_unique<TagLib::Ogg::Vorbis::File>(&h->stream,false);h->xiph=f->tag();h->file=std::move(f); }
   else return 3;
   h->family=2;
  } else if(prefix("ID3",3) || (magic.size()>=2 && static_cast<uint8_t>(magic[0])==255 && (static_cast<uint8_t>(magic[1])&0xe6)==0xe2)) {
   // Only offset-zero ID3 is validated and snapshotted. Fast discovery below
   // prevents TagLib from scanning junk prefixes for an unvalidated ID3 tag.
   // For untagged streams, reject reserved/free-format initial headers rather
   // than treating the two-byte sync dispatch predicate as a valid header.
   if(!prefix("ID3",3)) {
    if(magic.size()<4)throw Unsupported{};
    const auto b1=static_cast<uint8_t>(magic[1]), b2=static_cast<uint8_t>(magic[2]);
    if(((b1>>3)&3)==1 || ((b1>>1)&3)==0 || (b2>>4)==0 || (b2>>4)==15 || ((b2>>2)&3)==3)throw Unsupported{};
   }
   const auto length=static_cast<uint64_t>(h->stream.length());
   const bool id3v1=length>=128 && read_at(h->stream,length-128,3).startsWith("TAG");
   const uint64_t trailer=id3v1?160:32;
   if(length>=trailer && read_at(h->stream,length-trailer,8).startsWith("APETAGEX"))return 3;
   h->stream.seek(0);
   auto f=std::make_unique<TagLib::MPEG::File>(&h->stream,false,TagLib::AudioProperties::Fast);
   if(f->firstFrameOffset()<0)return 2;
   if(f->hasID3v1Tag() && !f->hasID3v2Tag())return 3;
   h->id3=f->ID3v2Tag();h->file=std::move(f);h->family=1;
  } else return 2; // includes ADTS: never admit a newly supported AAC tag read
  if(!h->file->isValid()) return 2;
  *out=h.release();return 0;
 } catch(const Unsupported&) {return 3;} catch(const InputFailure&) {return 4;} catch(...) {return 5;}
}
extern "C" int32_t sm_get(SmHandle* h, uint32_t field, SmResult* out) {
 if(!out) return 1;
 out->data=nullptr;out->len=0;
 if(!h || field>5) return 1;
 try {
  TagLib::StringList strings; bool integer=false;
  const char* id3keys[]={"TPE1","TPE2","TIT2","TALB","TCON","TRCK"};
  const char* keys[]={"ARTIST","ALBUMARTIST","TITLE","ALBUM","GENRE","TRACKNUMBER"};
  if(h->family==1) {
   const auto it=h->raw_id3.find(id3keys[field]);
   std::vector<uint8_t> bytes;u32(bytes,1);u32(bytes,it==h->raw_id3.end()?0:1);
   if(it!=h->raw_id3.end())add(bytes,10,std::string(it->second.data(),it->second.size()));
   auto allocation=std::make_unique<uint8_t[]>(bytes.size());std::copy(bytes.begin(),bytes.end(),allocation.get());
   out->len=bytes.size();out->data=allocation.release();return 0;
  } else if(h->family==2 && h->xiph) {
   const auto& map=h->xiph->fieldListMap();auto it=map.find(keys[field]);if(it!=map.end()) strings=it->second;
  } else if(h->family==3 && h->ape) {
   const auto& map=h->ape->itemListMap();auto it=map.find(field==5?"TRACK":field==1?"ALBUM ARTIST":keys[field]);
   if(it!=map.end()) { if(it->second.type()!=TagLib::APE::Item::Text) throw Unsupported{}; strings=it->second.values(); }
  } else if(h->family==4 && h->mp4) {
   const char* mp4keys[]={"\xc2\xa9" "ART","aART","\xc2\xa9" "nam","\xc2\xa9" "alb","\xc2\xa9" "gen","trkn"};
   const auto& map=h->mp4->itemMap();auto it=map.find(TagLib::String(mp4keys[field],TagLib::String::UTF8));
   if(it!=map.end()) {
    if(field==5) {strings.append(TagLib::String::number(it->second.toIntPair().first));integer=true;}
    else strings=it->second.toStringList();
   }
  }
  if(strings.size()>4096) throw InputFailure{};
  std::vector<uint8_t> bytes;u32(bytes,h->family);u32(bytes,strings.size());
  for(const auto& s:strings) add(bytes,integer?2:1,s.to8Bit(true));
  auto allocation=std::make_unique<uint8_t[]>(bytes.size());std::copy(bytes.begin(),bytes.end(),allocation.get());
  out->len=bytes.size();out->data=allocation.release();return 0;
 } catch(const Unsupported&) {return 3;} catch(const InputFailure&) {return 4;} catch(...) {return 5;}
}
extern "C" void sm_result_free(SmResult* result) {try {if(result){delete[] result->data;result->data=nullptr;result->len=0;}} catch(...) {}}
extern "C" void sm_close(SmHandle* h) {try {delete h;} catch(...) {}}
