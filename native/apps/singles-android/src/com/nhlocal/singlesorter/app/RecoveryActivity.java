package com.nhlocal.singlesorter.app;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.os.CancellationSignal;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Spinner;
import android.widget.TextView;
import android.widget.AdapterView;
import com.nhlocal.singlesorter.platform.SafReadStorage;
import java.util.ArrayList;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import org.json.JSONArray;
import org.json.JSONObject;

/** Current read-only inspection and explicit retention consent; never replays old effects. */
public final class RecoveryActivity extends Activity {
 private final ExecutorService worker=Executors.newSingleThreadExecutor();
 private final CancellationSignal cancelled=new CancellationSignal();
 private volatile boolean closed;private boolean busy;private Spinner choices;private Button inspect;private TextView status,details;
 private CheckBox consent;private Button accept;private String reviewedId,reviewedDigest;
 private String[] ids=new String[0];private boolean[] needsParent=new boolean[0];private Button parentPicker;private TextView parentPurpose;private SafReadStorage parentStorage;private Uri selectedParent;private String selectedParentId,pickerId;private long pickerTicket;
 @Override public void onCreate(Bundle state){
  super.onCreate(state);parentStorage=new SafReadStorage(this);LinearLayout page=new LinearLayout(this);page.setOrientation(LinearLayout.VERTICAL);page.setPadding(20,12,20,12);
  Button back=new Button(this);back.setText("חזרה");back.setOnClickListener(v->finish());page.addView(back);
  TextView title=new TextView(this);title.setText("בדיקת ביצוע קודם");title.setTextSize(22);page.addView(title);
  TextView explanation=new TextView(this);explanation.setText("נבדוק מה נרשם ומה נמצא כעת בתיקיות. הבדיקה אינה משנה קבצים או מנסה לבצע שוב. קובץ שנמצא כעת אינו מוכיח שהפעולה הקודמת הסתיימה.");page.addView(explanation);
  choices=new Spinner(this);page.addView(choices);inspect=new Button(this);inspect.setText("בדוק את הקבצים כעת");inspect.setEnabled(false);inspect.setOnClickListener(v->inspect());page.addView(inspect);
  parentPurpose=new TextView(this);parentPurpose.setText("ייתכן שתיקיית המקור עצמה הוסרה. יש לבחור מחדש את תיקיית האב שנרשמה, לצורך בדיקת הרשימה בלבד. מעבר למקור וליעדים שנרשמו, לא נבדוק תוכן של תיקיות שכנות. לא נמחק או ננסה להעביר שוב.");parentPurpose.setVisibility(android.view.View.GONE);page.addView(parentPurpose);
  parentPicker=new Button(this);parentPicker.setText("בחירת תיקיית האב לבדיקה בלבד");parentPicker.setVisibility(android.view.View.GONE);parentPicker.setOnClickListener(v->pickParent());page.addView(parentPicker);
  status=new TextView(this);status.setText("קורא את רישום הפעולות…");page.addView(status);
  ScrollView scroll=new ScrollView(this);details=new TextView(this);details.setTextIsSelectable(true);scroll.addView(details);page.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
  consent=new CheckBox(this);consent.setText("בדקתי את המיקומים. אני מבין/ה שחלק מהפעולות אינן ודאיות, ומשאיר/ה את הקבצים והיומן כפי שהם. אבנה תצוגה מקדימה חדשה לפני ביצוע נוסף.");consent.setEnabled(false);page.addView(consent);
  accept=new Button(this);accept.setText("שמור את האישור ואפשר תכנון חדש");accept.setEnabled(false);consent.setOnCheckedChangeListener((button,checked)->accept.setEnabled(checked&&!busy&&reviewedDigest!=null));accept.setOnClickListener(v->acceptReview());page.addView(accept);setContentView(page);
  choices.setOnItemSelectedListener(new AdapterView.OnItemSelectedListener(){@Override public void onItemSelected(AdapterView<?> adapter,android.view.View view,int position,long item){if(busy)return;selectedParent=null;selectedParentId=null;reviewedId=null;reviewedDigest=null;consent.setChecked(false);consent.setEnabled(false);accept.setEnabled(false);boolean required=position>=0&&position<needsParent.length&&needsParent[position];parentPicker.setVisibility(required?android.view.View.VISIBLE:android.view.View.GONE);parentPurpose.setVisibility(required?android.view.View.VISIBLE:android.view.View.GONE);}@Override public void onNothingSelected(AdapterView<?> adapter){}});
  worker.execute(()->{
   ArrayList<String> found=new ArrayList<>(),labels=new ArrayList<>();ArrayList<Boolean> parents=new ArrayList<>();String error=null;
   try(DiagnosticReadGate.Lease lease=ProviderExecutionOwner.beginDiagnosticRead()){
    lease.check();RetainedJournals.scan(this,cancelled::isCanceled,record->{if(record.attention){found.add(record.id);parents.add(record.authority().has("album_root_parent"));labels.add("פעולה "+found.size()+": "+record.acknowledged+" נרשמו עם תוצאה סופית, "+record.unresolved+" דורשות בירור"+(record.duetUnresolved>0?"; "+record.duetUnresolved+" מחיקות מקור דואט לא הושלמו":""));}});
   }catch(Exception failure){error="לא ניתן לקרוא כעת את רישום הפעולות. אפשר לשמור יומן לבדיקה דרך ההגדרות המתקדמות.";}
   final String failure=error;runOnUiThread(()->{if(closed)return;ids=found.toArray(new String[0]);needsParent=new boolean[parents.size()];for(int n=0;n<needsParent.length;n++)needsParent[n]=parents.get(n);choices.setAdapter(new ArrayAdapter<>(this,android.R.layout.simple_spinner_dropdown_item,labels));inspect.setEnabled(failure==null&&ids.length>0);status.setText(failure!=null?failure:ids.length==0?"אין פעולות שממתינות לבדיקה.":"בחר פעולה ובדוק את הקבצים.");});
  });
 }
 private void inspect(){
  if(closed||busy||choices.getSelectedItemPosition()<0)return;final String id=ids[choices.getSelectedItemPosition()];if(needsParent[choices.getSelectedItemPosition()]&&(selectedParent==null||!id.equals(selectedParentId))){status.setText("יש לבחור במפורש את תיקיית האב לבדיקה בלבד.");return;}final Uri parent=selectedParent;busy=true;parentPicker.setEnabled(false);reviewedId=null;reviewedDigest=null;consent.setChecked(false);consent.setEnabled(false);accept.setEnabled(false);inspect.setEnabled(false);choices.setEnabled(false);details.setText("");status.setText("בודק את הקבצים בתיקיות… אפשר לחזור כדי לעצור.");
  worker.execute(()->{
   JSONObject result=null;String error=null;
   try(DiagnosticReadGate.Lease lease=ProviderExecutionOwner.beginDiagnosticRead()){
    lease.check();RetainedJournals.Record[] selected={null};RetainedJournals.scan(this,cancelled::isCanceled,record->{if(record.id.equals(id))selected[0]=record;});
    if(selected[0]==null||!selected[0].attention)throw new java.io.IOException("REVIEW_NOT_AVAILABLE");result=RecoveryInspection.inspect(this,selected[0],cancelled,parent);
   }catch(Exception failure){error="הבדיקה לא הושלמה. הבדיקה הזו לא שינתה קבצים. יש לבדוק שהרשאות התיקיות עדיין זמינות ולשמור את היומן.";}
   final String failure=error;final JSONObject review=result;runOnUiThread(()->{if(closed)return;busy=false;parentPicker.setEnabled(true);inspect.setEnabled(true);choices.setEnabled(true);if(failure!=null){status.setText(failure);return;}try{render(review);choices.setEnabled(false);reviewedId=id;reviewedDigest=RetainedEffectsAcceptance.digest(review);consent.setEnabled(true);}catch(Exception malformed){status.setText("לא ניתן להציג את הבדיקה. הבדיקה הזו לא שינתה קבצים.");}});
  });
 }
 private void acceptReview(){
  if(closed||busy||!consent.isChecked()||reviewedId==null||reviewedDigest==null)return;
  final String id=reviewedId,digest=reviewedDigest;final Uri parent=selectedParent;busy=true;parentPicker.setEnabled(false);accept.setEnabled(false);inspect.setEnabled(false);choices.setEnabled(false);consent.setEnabled(false);status.setText("בודק שוב ושומר את האישור…");
  worker.execute(()->{
   boolean success=false;try{RetainedEffectsAcceptance.accept(this,id,digest,true,cancelled,parent);success=true;}catch(Exception failure){/* Fixed user-facing status; preserve all receipts and historical effects. */}
   final boolean saved=success;runOnUiThread(()->{if(closed)return;busy=false;parentPicker.setEnabled(true);reviewedId=null;reviewedDigest=null;consent.setChecked(false);inspect.setEnabled(true);choices.setEnabled(true);
    status.setText(saved?"האישור נשמר. הפעולות הקודמות נשארו כפי שתועדו, כולל אי-הוודאות. חזור ובנה תצוגה מקדימה חדשה; לא בוצע ניסיון חוזר ולא נוקו קבצים.":"שמירת האישור לא אומתה. ייתכן שנשמר אישור מלא או חלקי; אפשר לשמור את היומן לבדיקה. יש לבדוק שוב ולבנות תצוגה מקדימה חדשה. קבצים ויומנים קיימים לא נוקו.");});
  });
 }
 private void render(JSONObject review)throws Exception{
  JSONArray observations=review.getJSONArray("observations");int files=0,directories=0,unknown=0;StringBuilder text=new StringBuilder();
  if(review.has("source_parent_observation")){JSONObject parent=review.getJSONObject("source_parent_observation");text.append("בדיקת תיקיית האב: ").append(AlbumParentRecovery.description(parent.getString("status"))).append('\n');if(!parent.getString("status").equals("source_present")&&!parent.getString("status").equals("source_absent"))unknown++;}
  for(int i=0;i<observations.length();i++){
   JSONObject row=observations.getJSONObject(i);String state=row.getString("status");
   if(state.equals("content_observed"))files++;else if(state.equals("directory_observed"))directories++;else unknown++;
   String name=row.optString("name",row.optString("recorded_name",""));if(name.isEmpty())name="הפניה קודמת "+(i+1);
   String explanation=state.equals("content_observed")?"התוכן נבדק כעת":state.equals("directory_observed")?"התיקייה נמצאת כעת":state.equals("content_not_checked")?"התוכן לא נבדק בגלל מגבלת גודל":"לא ניתן לקבוע מה נמצא כעת";
   String scope=row.optString("scope_role",row.optString("recorded_scope_role",""));String location=scope.equals("source")?"מקור":scope.equals("target")?"יעד":scope.equals("source_parent")?"תיקיית הורה (פרטי תיקייה בלבד)":"מיקום קודם";
   JSONArray associations=row.getJSONArray("original_associations");java.util.LinkedHashSet<String> labels=new java.util.LinkedHashSet<>();
   for(int n=0;n<associations.length();n++){JSONObject relation=associations.getJSONObject(n);String role=relation.getString("role");String label=role.equals("source")?"קובץ המקור":role.equals("retained_original")?"גיבוי של המקור":role.equals("stage")?"קובץ ביניים":role.equals("destination")?"יעד שנרשם":"מועמד לפי שם צפוי";
    if(relation.has("matches_original_content"))label+=relation.getBoolean("matches_original_content")?", התוכן זהה למקור שתועד":", התוכן שונה מהמקור שתועד";if(relation.has("matches_staged_content"))label+=relation.getBoolean("matches_staged_content")?", התוכן זהה לפלט התיקון שתועד":", התוכן שונה מפלט התיקון שתועד";labels.add((relation.has("duet_group")?"קבוצת דואט "+(relation.getInt("duet_group")+1):"פעולה "+(relation.getInt("operation")+1))+": "+label);}
   text.append(location).append(" / ").append(RecoveryDisplay.escape(row.optString("document_id",row.optString("recorded_document_id",name)))).append(": ").append(explanation).append('\n');
   for(String label:labels)text.append(label).append('\n');text.append('\n');RecoveryDisplay.requireBound(text);
  }
  JSONArray intents=review.getJSONArray("intent_candidates");
  for(int i=0;i<intents.length();i++){JSONObject intent=intents.getJSONObject(i);String state=intent.getString("status");
   if(!state.equals("named_candidate_observed")){unknown++;text.append("שם צפוי: ").append(RecoveryDisplay.escape(intent.getString("intended_name"))).append(" בתיקייה ").append(RecoveryDisplay.escape(displayParent(intent.getString("recorded_parent")))).append(state.equals("ambiguous_name")?"; נמצאו כמה מועמדים, אין שיוך ודאי":"; לא נמצא מועמד שניתן לבדוק בוודאות").append('\n');}
  }
  JSONArray albumSubtrees=review.optJSONArray("album_subtree_observations");
  if(albumSubtrees!=null)for(int i=0;i<albumSubtrees.length();i++){JSONObject subtree=albumSubtrees.getJSONObject(i);String state=subtree.getString("status");if(!state.equals("names_currently_match"))unknown++;
   text.append("תכולת תיקיית אלבום / ").append(RecoveryDisplay.escape(displayParent(subtree.getString("recorded_uri")))).append(": ").append(AlbumRecoveryProjection.description(state,subtree.optInt("missing_names",0),subtree.optInt("current_children",0),subtree.optInt("expected_children",0))).append('\n');RecoveryDisplay.requireBound(text);}
  JSONArray duetDestinations=review.optJSONArray("duet_destination_observations");
  if(duetDestinations!=null){
   text.append("ברישום הדואטים הקודם: ").append(review.getInt("historical_duet_retired")).append(" מחיקות מקור אומתו, ").append(review.getInt("historical_duet_no_action")).append(" מקורות נשמרו ללא פעולה, ").append(review.getInt("historical_duet_unresolved")).append(" מחיקות מקור לא הושלמו.\n");
   for(int i=0;i<duetDestinations.length();i++){JSONObject row=duetDestinations.getJSONObject(i);text.append("יעד דואט נדרש ").append(row.getInt("operation")+1).append(": ").append(row.optBoolean("matches_recorded_destination",false)?"השם המתוכנן מפנה כעת ליעד שנרשם":"לא אומתה התאמה בין השם המתוכנן ליעד שנרשם").append('\n');}
  }
  RecoveryDisplay.requireBound(text);
  status.setText("ברישום הקודם: "+review.getInt("historical_acknowledged")+" פעולות נרשמו עם תוצאה סופית, "+review.getInt("historical_unresolved")+" דורשות בירור. כעת נבדקו "+files+" קבצים ונמצאו "+directories+" תיקיות; "+unknown+" הפניות לא נבדקו במלואן. עדיין לא אושר ביצוע חדש בתיקיות האלה.");details.setText(text.toString());
 }
 private String displayParent(String value){try{android.net.Uri uri=android.net.Uri.parse(value);if(ProviderReferences.AUTHORITY.equals(uri.getAuthority())&&android.provider.DocumentsContract.isDocumentUri(this,uri))return android.provider.DocumentsContract.getDocumentId(uri);}catch(RuntimeException invalid){/* Display only, never resolve a malformed reference. */}return "מיקום שלא ניתן לזהות";}
 private void pickParent(){
  if(closed||busy||choices.getSelectedItemPosition()<0||!needsParent[choices.getSelectedItemPosition()])return;pickerId=ids[choices.getSelectedItemPosition()];reviewedId=null;reviewedDigest=null;consent.setChecked(false);consent.setEnabled(false);accept.setEnabled(false);busy=true;inspect.setEnabled(false);choices.setEnabled(false);parentPicker.setEnabled(false);
  try{pickerTicket=parentStorage.beginPicker();Intent intent=SafReadStorage.treePickerIntent();intent.putExtra(Intent.EXTRA_TITLE,"בחירת תיקיית האב לבדיקה בלבד");startActivityForResult(intent,140);}catch(RuntimeException failure){try{if(pickerTicket!=0)parentStorage.finishPicker(pickerTicket,RESULT_CANCELED,null,cancelled);}catch(Exception ignored){}pickerTicket=0;busy=false;inspect.setEnabled(true);choices.setEnabled(true);parentPicker.setEnabled(true);status.setText("לא ניתן לפתוח את בחירת תיקיית האב. לא בוצע שינוי בקבצים.");}
 }
 @Override protected void onActivityResult(int request,int result,Intent data){
  super.onActivityResult(request,result,data);if(request!=140||closed||pickerTicket==0)return;final long ticket=pickerTicket;final String id=pickerId;pickerTicket=0;
  worker.execute(()->{Uri parent=null;boolean wasCancelled=false;String error=null;try(DiagnosticReadGate.Lease lease=ProviderExecutionOwner.beginDiagnosticRead()){
   lease.check();cancelled.throwIfCanceled();SafReadStorage.Entry choice=parentStorage.finishPicker(ticket,result,data,cancelled);if(choice==null)wasCancelled=true;else{RetainedJournals.Record[] record={null};RetainedJournals.scan(this,cancelled::isCanceled,value->{if(id.equals(value.id))record[0]=value;});if(record[0]==null||!record[0].attention)throw new java.io.IOException("REVIEW_NOT_AVAILABLE");JSONObject evidence=AlbumParentRecovery.evidence(record[0].authority(),cancelled::isCanceled);if(evidence==null)throw new java.io.IOException("ALBUM_PARENT_REVIEW_BINDING");String[] provenance=parentStorage.provenance(choice.token);Uri selected=Uri.parse(provenance[0]);AlbumRootParent.inspectMembership(this,evidence,selected,cancelled);parent=selected;}
  }catch(Exception failure){error="בחירת תיקיית האב לא אומתה. יש לבחור את תיקיית האב המדויקת שנרשמה; לא בוצע שינוי בקבצים.";}
  final Uri selected=parent;final boolean dismissed=wasCancelled;final String failure=error;runOnUiThread(()->{if(closed)return;busy=false;inspect.setEnabled(true);choices.setEnabled(true);parentPicker.setEnabled(true);if(selected!=null){selectedParent=selected;selectedParentId=id;status.setText("תיקיית האב נבחרה לבדיקה בלבד. כעת אפשר לבדוק את הקבצים.");}else if(failure!=null){selectedParent=null;selectedParentId=null;status.setText(failure);}else if(dismissed)status.setText("בחירת תיקיית האב בוטלה; לא בוצע שינוי בקבצים.");});});
 }
 @Override protected void onDestroy(){closed=true;cancelled.cancel();worker.execute(()->{try{if(parentStorage!=null)parentStorage.close();}catch(Exception ignored){}});worker.shutdown();super.onDestroy();}
}
