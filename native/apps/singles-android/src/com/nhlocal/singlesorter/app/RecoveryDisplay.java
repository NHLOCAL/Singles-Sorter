package com.nhlocal.singlesorter.app;
/** Unambiguous display-only escaping; raw provider references remain untouched for binding. */
final class RecoveryDisplay {
 private RecoveryDisplay(){}
 static String escape(String raw){
  if(raw==null||raw.length()>8192)throw new IllegalArgumentException("recovery display bound");StringBuilder out=new StringBuilder();
  for(int i=0;i<raw.length();){int point=raw.codePointAt(i);i+=Character.charCount(point);int kind=Character.getType(point);
   if(point=='\\')out.append("\\\\");
   else if(kind==Character.CONTROL||kind==Character.FORMAT||kind==Character.SURROGATE||kind==Character.LINE_SEPARATOR||kind==Character.PARAGRAPH_SEPARATOR)out.append(String.format(java.util.Locale.ROOT,point<=65535?"\\u%04X":"\\U%08X",point));
   else out.appendCodePoint(point);
  }return out.toString();
 }
 static void requireBound(StringBuilder text){if(text.length()>524288)throw new IllegalArgumentException("recovery display total bound");}
}
