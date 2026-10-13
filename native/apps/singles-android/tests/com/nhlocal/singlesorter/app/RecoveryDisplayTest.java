package com.nhlocal.singlesorter.app;
final class RecoveryDisplayTest {
 public static void main(String[] args){String[][] cases={{"שלום/song.wav","שלום/song.wav"},{"a\nb","a\\u000Ab"},{"a\tb","a\\u0009b"},{"x\u202Ey","x\\u202Ey"},{"x\u2066y\u2069","x\\u2066y\\u2069"},{"א\u200Fב","א\\u200Fב"},{"\\u202E","\\\\u202E"},{"x\u0000y","x\\u0000y"},{"😀","😀"},{"\uD800","\\uD800"},{"\u007F","\\u007F"},{"a\u2028b","a\\u2028b"},{"a\u2029b","a\\u2029b"}};int checks=0;
  for(String[] value:cases){if(!RecoveryDisplay.escape(value[0]).equals(value[1]))throw new AssertionError(value[0]);checks++;}
  try{RecoveryDisplay.escape(new String(new char[8193]));throw new AssertionError();}catch(IllegalArgumentException expected){checks++;}
  try{RecoveryDisplay.requireBound(new StringBuilder(new String(new char[524289])));throw new AssertionError();}catch(IllegalArgumentException expected){checks++;}
  System.out.println(checks+" recovery Hebrew/Latin/control/bidi display checks passed");
 }
}
