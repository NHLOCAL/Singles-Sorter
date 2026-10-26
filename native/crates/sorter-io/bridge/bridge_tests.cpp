// Deterministic malformed-input/ABI ownership harness; no writes to media.
#include "taglib_bridge.h"
#include <cassert>
#include <cstring>
#include <fstream>
#include <iterator>
#include <vector>
#include <limits>
static int32_t read_at(void* context,uint64_t at,uint8_t* out,size_t n,size_t* got){
 const auto& v=*static_cast<const std::vector<uint8_t>*>(context);
 if(at>v.size()||n>v.size()-at)return 1;
 std::memcpy(out,v.data()+at,n);*got=n;return 0;
}
static void exercise(std::vector<uint8_t>& bytes){
 SmInput in{1,sizeof(SmInput),&bytes,read_at,bytes.size()};SmHandle* h=nullptr;
 const auto status=sm_open(&in,&h);assert(status>=0&&status<=5);
 if(status==0){assert(h);for(uint32_t f=0;f<6;++f){SmResult r{};const auto status=sm_get(h,f,&r);assert(status>=0&&status<=5);if(status==0){assert(r.data&&r.len>=8&&r.len<=16*1024*1024);}sm_result_free(&r);sm_result_free(&r);}SmResult bad{};assert(sm_get(h,99,&bad)==1);sm_close(h);}else assert(!h);
}
int main(int argc,char**argv){
 SmHandle*h=reinterpret_cast<SmHandle*>(1);assert(sm_open(nullptr,&h)==1&&!h);assert(sm_open(nullptr,nullptr)==1);sm_close(nullptr);sm_result_free(nullptr);
 std::vector<uint8_t> bytes(100);SmInput bad{9,sizeof(SmInput),&bytes,read_at,bytes.size()};assert(sm_open(&bad,&h)==1&&!h);bad.version=1;bad.struct_size=0;assert(sm_open(&bad,&h)==1);bad.struct_size=sizeof(SmInput);bad.length=std::numeric_limits<uint64_t>::max();assert(sm_open(&bad,&h)==1);
 for(int i=1;i<argc;++i){std::ifstream file(argv[i],std::ios::binary);assert(file);std::vector<uint8_t> original((std::istreambuf_iterator<char>(file)),{});exercise(original);for(size_t n=0;n<original.size();n=n<64?n+1:n*2){auto truncated=std::vector<uint8_t>(original.begin(),original.begin()+n);exercise(truncated);}}
 uint64_t state=1729;
 for(size_t i=0;i<3000;++i){state=state*6364136223846793005ULL+1;bytes.resize(state%4096);for(auto& b:bytes){state=state*6364136223846793005ULL+1;b=state>>32;}if(bytes.size()>10 && i%2==0){std::memcpy(bytes.data(),"ID3\4\0\0",6);}exercise(bytes);}
}
