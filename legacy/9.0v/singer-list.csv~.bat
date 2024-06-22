@echo off & pushd %~dp0
powershell -noprofile -c "$f=[io.file]::ReadAllText('%~f0') -split ':bat2file\:.*';iex ($f[1]);X 1;"
del %0& exit /b

:bat2file: Compressed2TXT v5.3
Add-Type -Language CSharp -TypeDefinition @"
 using System.IO; public class BAT85{ public static void Decode(string tmp, string s) { MemoryStream ms=new MemoryStream(); n=0;
 byte[] b85=new byte[255]; string a85="0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!#$&()+,-./;=?@[]^_{|}~";
 int[] p85={52200625,614125,7225,85,1}; for(byte i=0;i<85;i++){b85[(byte)a85[i]]=i;} bool k=false;int p=0; foreach(char c in s){
 switch(c){ case'\0':case'\n':case'\r':case'\b':case'\t':case'\xA0':case' ':case':': k=false;break; default: k=true;break; }
 if(k){ n+= b85[(byte)c] * p85[p++]; if(p == 5){ ms.Write(n4b(), 0, 4); n=0; p=0; } } }         if(p>0){ for(int i=0;i<5-p;i++){
 n += 84 * p85[p+i]; } ms.Write(n4b(), 0, p-1); } File.WriteAllBytes(tmp, ms.ToArray()); ms.SetLength(0); }
 private static byte[] n4b(){ return new byte[4]{(byte)(n>>24),(byte)(n>>16),(byte)(n>>8),(byte)n}; } private static long n=0; }
"@; function X([int]$r=1){ $tmp="$r._"; echo "`n$r.."; [BAT85]::Decode($tmp, $f[$r+1]); expand -R $tmp -F:* .; del $tmp -force }

:bat2file: singer-list.csv~
::O/bZg00000I12y)00000EC2ui000000|5a50RR9100000OaK4@0RRIPv@2fi00000001?r4=5l2b7]j8WpXWSX?+WgV{?+[#=$8|;O/MRTY!L#0uVwg003kJG,SRy
::4Mc+cN4Ky;$_AMN{QE{SS4^^?i&Z]59hrat0RW5+-}RlbCxKg^(1vAU+4NC@dWW3^#m;{i+&]RbTWx5z,(;(8CmyUFyW=OF=^rQWkkut7c^I{JmD$bXrd.y_-@LnW
::sCCBXV1(O;{_b=V01N/C3/^-1OhKe4H16m&q48J[carz^|M!x#bY+Djjc~vI$;?_9Ytm4plo4ZpN2ro2+iM3PyZM!lO=FMC-+ZhbC-gQt9mp)#NdI!k/6r?Dn~_Vj
::6#o^7ZceTeEubTcDKz/-Zw3;(rlo+6.@[n#q|86c9o.8r_WpxPu}c#t_?~D0[.DE;[{CC.yHDIQ5kM./JT)Mxm1T5P8L[g|@F&RZ=S$l7hdT_KUAi#rWE8ye4l7BO
::yK;ZXf4h[{0VX?~)5L~GL?cMD[5#?b2pG!qe#Zbv$|!RBY,gWn.TUVBLtP|5h@x2&yCjq$5-MV2eLnQ#8#gNWUu8ETl22kFjS}50DqUp~4t6/MbXL}NmJu{)eB|ZT
::@id{ukOotbL]w^_98MGW;2T.&D7[lkzwtA3^?L@hY6r@v.eg-[YT!yIFkJ{|irgQikxs}T-3|{GMQEhKDsQv0L+C^j!&k;VqdPyDrl;=l1Y{Lvgwn+rGhkJd14{e?
::lsq5Kq0QW=vx=)42t7..-FjoM8y06r[l,r=mt.!9fJ_Lb!HIJi?6I2-#i7Ad!W=5jmaY;SSqmOUm},q4+v]It3U+{QhE067G6GdeiXy^((1nGbYPzXsMx0V&B8@o0
::=aiaej_RqqGohx=bsJ0;n{)ap2}OgQ&4wwwi@s#ZWE5TcWN9v~;|tQu@1h/mVkjAk9e)hH!i//#A|[yd_UOXdpE;hr71E^vf_a.mvnrP~Rlk9BGZY6qdr|]UOzAeN
::fGf#X.USxq{}UGvjC(?1b^y7h]9ahx0.4M=]{K!]a(Cn@H1|rnV6@+Fx}!&u.8rATA+3pa^ty/[ZNZuzTU^[kMM0U/C1FNauL&,)10IKcQ~xqYA9-[Itm}r.?T=oY
::yv#aXl;1XEqRO9-R6uznrZl@YRAIEb&LwQ&3zgX6{S@olbVTMZ#c3$2_ejR~s2-@U,P/y##@n!Lx|m,5,Spt,6yezg!ERAZrXw|=A2qg@jJpI)H$j$Fbe?(-w;&/O
::pr;I{]~(OcEZM$?[]A6NXBO}Zqu1Ed3g~zw?rky=PzjO|)O~op[_)l-!jwJFg!&-v6=iygryc.~S(qP[fYc(B0P2C9X,Te=O08]CQgwf5awwYlG/etC^OnbzG~t;N
::oo75QWCZk]oa?oPwftFx3F!Q,kL,zZtRq]Z#/1WTIarz1Lg#Ztj}BIiDB|l($UKCd&36#iLv/^D1q=rtercCAk_eY;P0w!3GDX9w;PW+WTjn4,U[(DD=eu!DG/17j
::PxLUHFYKWx-AH~8CrP_0G=TJBxK=mLtj,/+uur?~@Vf]xpNxISU/r(+nPgkqkw@0mzxBq30,?=MnYOYEH{91sfb^DAY,VY-AJt12aBvfHq5wNbkGgyrL/G3-UXSos
::,_nmvyxav/dE3^NqPtc&OD5TFZ/pB;grDglMt9=#cFNfjBed$8uRG@LlfN(W^7oI.[{HZ=cT1(uMKB$_-cpMWebTg[qM5^bvQ[Tr9@_|$e|_haV;a&9dnmBf(b#8I
::PH~PHEvz&B1mibb/VwEaho|zssj1/jRIlsTnNVPoTm($8)p1_{lL@0iBa$kte~u(HS)5pzNT)9-R^|TncOj89d!6q|qOFetK0s@Q2=[HluqSr/x-_5MLW.n0-JQcV
::ft9u^AKyV++t(8UBLG?J{+26O{xwK.@L]UFlLq}@6c7zI(GJH05zcEHvEBJ=0IgcX_wBA-d^V}OHOanD9(cSJNP&K!nQEEr/|yQ+HK=[({G-f.JM2QWsyCuy@!!VW
::F9Cz4u2zK6bmmxR8uH!81b&3[@|jyI76r(|VBS&,,4JIBqBAa9Ybx.en@/ffrnE4TX&~79weT^AZ^a/);#Vuz,/gHl2+6SmlApiUKO.ka$$KKwHr~5w1XS#HX~#ph
::,$^+Qd&0SKP}!L33Hs_YxI}Pt_Gn^@cMBO8x-!Pry#2jt6kgWbmGcsypwLE3l6B.vBzx,cDo#-^[zga6;+@eUfU&|vRXlJ_bi^]Iju7bSRlmUG;555HQKrIO09/8I
::qkie;Og#3}wk}&0tL{c@+H9??MpzQ~Ap[k_tBb+OHR!lg0]bKXRQv((TZE/gD]_T|{t-}yaFbN]sv51z@^&2pk99|YJ..UDcoD1,#-mPZj{X;vO1cqF7}hmeqX4F~
::=Cbz5n;Hn&JZwrAV_VJv,tqV9JBzd+#9E;F0v9_iX)-o5#IHa/RH=X=dKBs=BAq]!+,[nbYqQ]]qhN4Dg_32HBd|5+;m._cYFm@+M,(sXn6N|]Bj7[)$d${S1)a^c
::p&iZf2xMqy]|oPYlbw?IFxT.7JPJQ85VCk;t/BfPxLu~9Vxv|MA;lnydqaZ}xAhN2;rbfvIg&6Hle?T2_.NBcPR/j..pZ{v83Iw?6x9MOK9d.mbB?_syDIoT,rMd/
::oKa{5ktsOe,3P1dE,uvDJPM~tr4Abb=Wh|zH8_UM.DRKU4T&{,^Up&GQNECo?Ww=|we+Jdw.wD/q8AO_wi9[3A}o_KNJsV~7KO,L$)$G0vx][a!u;A#D,w{&{~pq6
::]F{{n]D!]BNWkSsh~lT|/U2BrTpe;+RsMN+95=L$rGMb.xsW]dV)LUgw,G4F1#+U1YElKLfz?^Mz;{TY+bar&c}Hjw,d@^Kx$_tY^vl?k;6AxhXrTFy+UAlA_,z/t
::[$Ejd4C85Uya)](xGEd(JNgd5U^+pC6j-F&tLRTrp.HG^j_$SP8qE5xz)3h&wW[D~C-||N(Jl)G#q5Bq@Gb5kYK#n2TkL3_AmX)PTeD2H_$KwfNwo.&993If{l,9?
::)DV[$L^/ej58G}^jY3J~[0Wq3UmTr[dR^qrgmuW)a2prdu5X762~;Z(WbvkZyTG5-{FoDqP?9^U@!s}G3xA8j[=hCE-J$M_v)zy@c]AOa_o,2,;c[6g0lYQN0Tl&v
::7dHE+Z)z)BJgHzMlKB0MSVD.0NdAvG3c0f^FPsdTAp6g7ov(CH6lRK@D/k}O(8HfU_7G9ii#)fKD5|G!Rj&1$T/QnMiV&Jh$j!a#1xR5O8zji.GPE][w^}X0zhHZP
::qI?9tb4y_^F7,gojk6EM3Bwl]M;@h[uXDFc/qmJ~tjt,4fZ=}6O;fu0GgfW-hGJ.}r)M&30]5K)+hK=G;1dVL[vD$SSMRqMb/2m)Y^r7=B+J5fM}HtDm/P5KR?fTN
::7~u~w];Ejg76n4XBVK=4=Eu&(V{E&fhK?T+Is(5D-B]nxZGC?A/6m)CZq[0W^f?clAHKpkZ#x1Ut3WOsq^&th=$ecA6g&NY2up~MZ+yUa=qN3R5y41@5pYP7D[J-8
::.VbLjtO8NuK$Rv[J|lJ5N)r8)D#A60PKLDgufkXOp=a72-p&=TtJlH&rldS#-$?zLe2[Bh@G(!0HQ|mBnQO=&28aL{
:bat2file: end
