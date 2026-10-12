package com.nhlocal.singlesorter.app;

import android.app.Activity;
import android.os.Bundle;
import android.os.CancellationSignal;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Spinner;
import android.widget.TextView;
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
 private String[] ids=new String[0];
 @Override public void onCreate(Bundle state){
  super.onCreate(state);LinearLayout page=new LinearLayout(this);page.setOrientation(LinearLayout.VERTICAL);page.setPadding(20,12,20,12);
  Button back=new Button(this);back.setText("חזרה");back.setOnClickListener(v->finish());page.addView(back);
  TextView title=new TextView(this);title.setText("בדיקת ביצוע קודם");title.setTextSize(22);page.addView(title);
  TextView explanation=new TextView(this);explanation.setText("נבדוק מה נרשם ומה נמצא כעת בתיקיות. הבדיקה אינה משנה קבצים או מנסה לבצע שוב. קובץ שנמצא כעת אינו מוכיח שהפעולה הקודמת הסתיימה.");page.addView(explanation);
  choices=new Spinner(this);page.addView(choices);inspect=new Button(this);inspect.setText("בדוק את הקבצים כעת");inspect.setEnabled(false);inspect.setOnClickListener(v->inspect());page.addView(inspect);
  status=new TextView(this);status.setText("קורא את רישום הפעולות…");page.addView(status);
  ScrollView scroll=new ScrollView(this);details=new TextView(this);details.setTextIsSelectable(true);scroll.addView(details);page.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
  consent=new CheckBox(this);consent.setText("בדקתי את המיקומים. אני מבין/ה שחלק מהפעולות אינן ודאיות, ומשאיר/ה את הקבצים והיומן כפי שהם. אבנה תצוגה מקדימה חדשה לפני ביצוע נוסף.");consent.setEnabled(false);page.addView(consent);
  accept=new Button(this);accept.setText("שמור את האישור ואפשר תכנון חדש");accept.setEnabled(false);consent.setOnCheckedChangeListener((button,checked)->accept.setEnabled(checked&&!busy&&reviewedDigest!=null));accept.setOnClickListener(v->acceptReview());page.addView(accept);setContentView(page);
  worker.execute(()->{
   ArrayList<String> found=new ArrayList<>(),labels=new ArrayList<>();String error=null;
   try(DiagnosticReadGate.Lease lease=ProviderExecutionOwner.beginDiagnosticRead()){
    lease.check();RetainedJournals.scan(this,cancelled::isCanceled,record->{if(record.attention){found.add(record.id);labels.add("פעולה "+found.size()+": "+record.acknowledged+" נרשמו כהושלמו, "+record.unresolved+" דורשות בירור");}});
   }catch(Exception failure){error="לא ניתן לקרוא כעת את רישום הפעולות. אפשר לשמור יומן לבדיקה דרך ההגדרות המתקדמות.";}
   final String failure=error;runOnUiThread(()->{if(closed)return;ids=found.toArray(new String[0]);choices.setAdapter(new ArrayAdapter<>(this,android.R.layout.simple_spinner_dropdown_item,labels));inspect.setEnabled(failure==null&&ids.length>0);status.setText(failure!=null?failure:ids.length==0?"אין פעולות שממתינות לבדיקה.":"בחר פעולה ובדוק את הקבצים.");});
  });
 }
 private void inspect(){
  if(closed||busy||choices.getSelectedItemPosition()<0)return;final String id=ids[choices.getSelectedItemPosition()];busy=true;reviewedId=null;reviewedDigest=null;consent.setChecked(false);consent.setEnabled(false);accept.setEnabled(false);inspect.setEnabled(false);choices.setEnabled(false);details.setText("");status.setText("בודק את הקבצים בתיקיות… אפשר לחזור כדי לעצור.");
  worker.execute(()->{
   JSONObject result=null;String error=null;
   try(DiagnosticReadGate.Lease lease=ProviderExecutionOwner.beginDiagnosticRead()){
    lease.check();RetainedJournals.Record[] selected={null};RetainedJournals.scan(this,cancelled::isCanceled,record->{if(record.id.equals(id))selected[0]=record;});
    if(selected[0]==null||!selected[0].attention)throw new java.io.IOException("REVIEW_NOT_AVAILABLE");result=RecoveryInspection.inspect(this,selected[0],cancelled);
   }catch(Exception failure){error="הבדיקה לא הושלמה. הבדיקה הזו לא שינתה קבצים. יש לבדוק שהרשאות התיקיות עדיין זמינות ולשמור את היומן.";}
   final String failure=error;final JSONObject review=result;runOnUiThread(()->{if(closed)return;busy=false;inspect.setEnabled(true);choices.setEnabled(true);if(failure!=null){status.setText(failure);return;}try{render(review);choices.setEnabled(false);reviewedId=id;reviewedDigest=RetainedEffectsAcceptance.digest(review);consent.setEnabled(true);}catch(Exception malformed){status.setText("לא ניתן להציג את הבדיקה. הבדיקה הזו לא שינתה קבצים.");}});
  });
 }
 private void acceptReview(){
  if(closed||busy||!consent.isChecked()||reviewedId==null||reviewedDigest==null)return;
  final String id=reviewedId,digest=reviewedDigest;busy=true;accept.setEnabled(false);inspect.setEnabled(false);choices.setEnabled(false);consent.setEnabled(false);status.setText("בודק שוב ושומר את האישור…");
  worker.execute(()->{
   boolean success=false;try{RetainedEffectsAcceptance.accept(this,id,digest,true,cancelled);success=true;}catch(Exception failure){/* Fixed user-facing status; preserve all receipts and historical effects. */}
   final boolean saved=success;runOnUiThread(()->{if(closed)return;busy=false;reviewedId=null;reviewedDigest=null;consent.setChecked(false);inspect.setEnabled(true);choices.setEnabled(true);
    status.setText(saved?"האישור נשמר. הפעולות הקודמות נשארו כפי שתועדו, כולל אי-הוודאות. חזור ובנה תצוגה מקדימה חדשה; לא בוצע ניסיון חוזר ולא נוקו קבצים.":"שמירת האישור לא אומתה. ייתכן שנשמר אישור מלא או חלקי; אפשר לשמור את היומן לבדיקה. יש לבדוק שוב ולבנות תצוגה מקדימה חדשה. קבצים ויומנים קיימים לא נוקו.");});
  });
 }
 private void render(JSONObject review)throws Exception{
  JSONArray observations=review.getJSONArray("observations");int files=0,directories=0,unknown=0;StringBuilder text=new StringBuilder();
  for(int i=0;i<observations.length();i++){
   JSONObject row=observations.getJSONObject(i);String state=row.getString("status");
   if(state.equals("content_observed"))files++;else if(state.equals("directory_observed"))directories++;else unknown++;
   String name=row.optString("name",row.optString("recorded_name",""));if(name.isEmpty())name="הפניה קודמת "+(i+1);
   String explanation=state.equals("content_observed")?"התוכן נבדק כעת":state.equals("directory_observed")?"התיקייה נמצאת כעת":state.equals("content_not_checked")?"התוכן לא נבדק בגלל מגבלת גודל":"לא ניתן לקבוע מה נמצא כעת";
   String scope=row.optString("scope_role",row.optString("recorded_scope_role",""));String location=scope.equals("source")?"מקור":scope.equals("target")?"יעד":"מיקום קודם";
   JSONArray associations=row.getJSONArray("original_associations");java.util.LinkedHashSet<String> labels=new java.util.LinkedHashSet<>();
   for(int n=0;n<associations.length();n++){JSONObject relation=associations.getJSONObject(n);String role=relation.getString("role");String label=role.equals("source")?"קובץ המקור":role.equals("stage")?"קובץ ביניים":role.equals("destination")?"יעד שנרשם":"מועמד לפי שם צפוי";
    if(relation.has("matches_original_content"))label+=relation.getBoolean("matches_original_content")?", התוכן זהה למקור שתועד":", התוכן שונה מהמקור שתועד";labels.add("פעולה "+(relation.getInt("operation")+1)+": "+label);}
   text.append(location).append(" / ").append(RecoveryDisplay.escape(row.optString("document_id",row.optString("recorded_document_id",name)))).append(": ").append(explanation).append('\n');
   for(String label:labels)text.append(label).append('\n');text.append('\n');RecoveryDisplay.requireBound(text);
  }
  JSONArray intents=review.getJSONArray("intent_candidates");
  for(int i=0;i<intents.length();i++){JSONObject intent=intents.getJSONObject(i);String state=intent.getString("status");
   if(!state.equals("named_candidate_observed")){unknown++;text.append("שם צפוי: ").append(RecoveryDisplay.escape(intent.getString("intended_name"))).append(" בתיקייה ").append(RecoveryDisplay.escape(displayParent(intent.getString("recorded_parent")))).append(state.equals("ambiguous_name")?"; נמצאו כמה מועמדים, אין שיוך ודאי":"; לא נמצא מועמד שניתן לבדוק בוודאות").append('\n');}
  }
  RecoveryDisplay.requireBound(text);
  status.setText("ברישום הקודם: "+review.getInt("historical_acknowledged")+" פעולות נרשמו כהושלמו, "+review.getInt("historical_unresolved")+" דורשות בירור. כעת נבדקו "+files+" קבצים ונמצאו "+directories+" תיקיות; "+unknown+" הפניות לא נבדקו במלואן. עדיין לא אושר ביצוע חדש בתיקיות האלה.");details.setText(text.toString());
 }
 private String displayParent(String value){try{android.net.Uri uri=android.net.Uri.parse(value);if(ProviderReferences.AUTHORITY.equals(uri.getAuthority())&&android.provider.DocumentsContract.isDocumentUri(this,uri))return android.provider.DocumentsContract.getDocumentId(uri);}catch(RuntimeException invalid){/* Display only, never resolve a malformed reference. */}return "מיקום שלא ניתן לזהות";}
 @Override protected void onDestroy(){closed=true;cancelled.cancel();worker.shutdownNow();super.onDestroy();}
}
