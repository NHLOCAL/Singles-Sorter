::���� ������� ��� ���� ����� ������ ������� ��� �����
::������� ����� ������ ����� ���� �� ����� ��� ������ ������ ��
::�����: nh.local11@gmail.com

@echo off
::������ �� ���, ���, ����� ����� �����
::���� ���� ������ ������
chcp 1255>nul
title %VER% ���� ��������
MODE CON COLS=80 lines=27
color f1
set VER=8.1

::����� ���� ���� ����� ����� ���� �� ����
if not exist "singer-list2.csv" call :creat-cvs

::����� �� ���� ���� ����� ������
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/new-ver-exist -o "%temp%\ver-exist-7.tmp"
if errorlevel 1 goto :call-num else (
set/p update=<"%temp%\ver-exist-7.tmp"
del "%temp%\ver-exist-7.tmp"
if %update% GTR %VER% goto :updating
)


:call-num
::����� ���� ������ ����� ��� �������
::���� ���� ����� ����� ���� ����
::������ �������� �� ����� ����� ������
if exist "%temp%\ver-exist-7.tmp" del "%temp%\ver-exist-7.tmp"
type "singer-list2.csv" | find /c ",">"%temp%\num-singer.tmp"
set /p ab=<"%temp%\num-singer.tmp"
if exist "%temp%\num-singer.tmp" del "%temp%\num-singer.tmp"

:sln-start
cls
set/a abc=%ab%

goto :mesader-singels

::��� ������ ����� �� ��� ������
:singer-list-new
cls
echo.
echo.
echo.
echo.[30m 
echo                             ����� ����� - �������� ����
echo                      ========================================= [34m 
echo.
echo.
echo                           [1] ���� ���� ����� ���� ������
echo                                 [2] ����� ������ �����
echo.
echo                                ���������� ���� ���
choice /c 12>nul
if errorlevel 2 goto mesader-singels
if errorlevel 1 "singer-list2.csv"


::���� �������� ����
:mesader-singels
cls
echo. [30m
echo          ____  _             _             ____             _
echo         / ___^|(_)_ __   __ _^| ^| ___  ___  / ___^|  ___  _ __^| ^|_ ___ _ __
echo         \___ \^| ^| '_ \ / _` ^| ^|/ _ \/ __^| \___ \ / _ \^| '__^| __/ _ \ '__^|
echo          ___) ^| ^| ^| ^| ^| (_^| ^| ^|  __/\__ \  ___) ^| (_) ^| ^|  ^| ^|^|  __/ ^|
echo         ^|____/^|_^|_^| ^|_^|\__, ^|_^|\___^|^|___/ ^|____/ \___/^|_^|   \__\___^|_^|
echo                        ^|___/                     
echo ================================================================================
echo                                 %VER% �������� ����
echo                                       ***** [34m 
echo.
echo.
echo.                           [0] ��� ������� ������ !���
echo                                 -----------------
echo                               [1] ��� ������ ��� ��
echo                          [2] ��� (�������� �������) �����
echo                             [3] ��� ���� ����� ������
echo                                  [4] ��� ?��� ��
echo.
echo                       !���������� ���� ������ ������ ���� ���
choice /c 01234>nul
if errorlevel 5 (
cls
echo. [30m
echo                                        ___
echo                                       ^|__ \
echo                                         / /
echo                                        ^|_^|
echo                                        (_^)
echo ================================================================================
echo                                  ?%VER% ����� ��� ��
echo                                        ***** [34m
echo.
echo                               ������� ���� ������ *
echo                           ���� ������� ����� ����� *
echo                            ������ ����� �� �� ����� *
echo.                          
echo                  ����� ���� �� ������� ������ ������ �������
echo               ���� ����� ����� _ ���� �� ����� ���� �� :������
echo             ����� ������ "���� ������" ,"����� ����" ��� �������
echo.
echo                            :����� nh.local11@gmail.com
echo.
echo                        ����� ������ ����� ����� ��� �� ���
pause>nul
goto :mesader-singels
)

if errorlevel 4 goto :singer-list-new
if errorlevel 3 goto :help
if errorlevel 2 goto :beginning
if errorlevel 1 goto :updating

:updating
cls
echo. [30m
echo                                        ___
echo                                       ^|__ \
echo                                         / /
echo                                        ^|_^|
echo                                        (_^)
echo ================================================================================[34m
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/%VER%%%2Bversion
echo.
echo.                                 1 ��� ��� ������
echo                              2 ��� ����� ������ �����
echo.
echo                              -----------------------
echo.                              %VER% ��� ������� �����
echo.
choice /c 12
if errorlevel 2 goto :mesader-singels
if errorlevel 1 (
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/SinglesSorter-up.bat -o "%~dp0\���� �������� %update%.bat"
cls
echo.[30m
echo                                        ___
echo                                       ^|   ^|
echo                                       \   /
echo                                        \_/
echo                                        ^(_^)
echo ================================================================================
echo.[34m
echo.
echo.                           !���� ��� %update% ���� !��� ���
timeout 7 | echo               ...��� ���� ����� ������ ����� ����� �� ����� ���� 
explorer "%~dp0"
cls & "%~dp0\���� �������� %update%.bat"

)

goto :mesader-singels

:help
cls
echo.[30m
echo                                ���� - �������� ����
echo ================================================================================
echo.[34m
echo            .����� ��� ������ ����� ���� �������� �� ���� ��� ������ ����
echo.
echo         .���� �� ������ ����� ���� ���� ������� �������� ������ �� ����� ��
echo             .���� ������ ����� ���� ������ ��� ����� ����� �� ���� ����
echo.
echo                  ����� ������� ������� �������� ���� ���� �� ����
echo            .������� ���� �� ����� ��� �� ������ ��������� �� ����� �����
echo                     ������� ������� �� ���� ��� ���� �� ���
echo                                !��� ������ !���
echo.
echo          .������ ���'�� ����� ����� ���� ����� ��� ������ ������ �� ���� 
echo                     !���� ������ �� ����� ������ !����� ����
echo.
echo                          ...����� ������ ������ ����    
echo                        mesader.singelim@gmail.com :����
echo.
echo                              1 ��� ���� ����� �����
echo                           2 ��� ����� ���� ���� ������
echo                           3 ��� ����� ���� ���� ������
echo                             4 ��� ����� ������ �����
choice /c 1234>nul
if errorlevel 4 goto :mesader-singels
if errorlevel 3 start https://mail.google.com/mail/u/0/?fs=1^&tf=cm^&source=mailto^&to=mesader.singelim@gmail.com & goto :mesader-singels
if errorlevel 2 (curl https://www.googleapis.com/drive/v3/files/1RJWxutr4oGNtL11vmsncVyfQ0jOvWQX1?alt=media^&key=AIzaSyDduW1Zbi2MIu8aMUMF6op72pJ1f0sPBi0 -o "%userprofile%\downloads\������ ����� ��������.pdf"
cls
echo.
echo.
echo                         !��� ������� ������� ��� �����
pause>nul
goto :mesader-singels
)
if errorlevel 1 start https://drive.google.com/file/d/1RJWxutr4oGNtL11vmsncVyfQ0jOvWQX1/preview & goto :mesader-singels


:wrong_path
echo.
echo                      !��� ����� �� ���� ��� !���� ���� �����
timeout 2 >nul
exit /b

:beginning
cls
set a=*���*����*.*
set c=��� ����
set d=1

echo.[30m
echo                                     _  __   __
echo                                    / ^| \ \  \ \
echo                                    ^| ^|  \ \  \ \
echo                                    ^| ^|  / /  / /
echo                                    ^|_^| /_/  /_/
echo ================================================================================
echo                                 %VER% �������� ����
echo                                       *****
echo                             ���� + 0 ����� ������ ������[34m
echo.
echo.
echo                ������ ���� ���� ������� ����� ���� ��� �� ����� ����
echo                       ���� ����� ����� ���� ��� - ��������
echo.
echo                         !���� ����� �� ����� ����� ����
echo.
set/p source_path=
::����� �� ���� 0 ����� ���� ������ �����
if [%source_path%] == [0] goto :mesader-singels
::���� ������ �������
for %%i in (%source_path%) do set source_path=%%~i
::����� �� ����� ����� ���� �� ���� �� ����
if not exist "%source_path%\" call :wrong_path & goto :beginning


:target_folder
cls
echo.[30m
echo                                  __    ____   __
echo                                  \ \  ^|___ \  \ \
echo                                   \ \   __) ^|  \ \
echo                                   / /  / __/   / /
echo                                  /_/  ^|_____^| /_/
echo ================================================================================
echo                                 %VER% �������� ����
echo                                       *****
echo                             ���� + 0 ����� ������ ������[34m
echo.
echo.
echo                   ������ ������ ���� ������ �� ������ ����� ����
echo                       ���� ����� ����� ���� ��� - ��������
echo.
echo                         !���� ����� �� ����� ����� ����
echo.
echo               ����+1 ����� ������ ������ ������� ����� ����� ������
echo.
set/p h=
::����� �� ���� 1 ����� ����� ��� ����� �������
if [%h%] == [1] md "������� �������" & set h="%~dp0������� �������"
::����� �� ���� 0 ����� ���� ������ �����
if [%h%] == [0] goto :mesader-singels
::���� ������ �������
for %%i in (%h%) do set h=%%~i
::����� �� ����� ����� ���� �� ���� �� ����
if not exist "%h%\" call :wrong_Path & goto :target_folder

::����� ������ ������� ������ �����
cd /d "%source_path%"

::����� ������ ����� ������ ������
set "clear_heb=����"
set cm_heb=�����
set "abc_heb=���� ��"
set c_or_m=move
set "sing_heb=���� ��"
set "fixed_heb=���� ��"
::����� ������� ����� ������
:options
cls
echo.[30m
echo                                 __   __    _____
echo                                 \ \  \ \  ^|___ /
echo                                  \ \  \ \   ^|_ \
echo                                  / /  / /  ___) ^|
echo                                 /_/  /_/  ^|____/
echo ================================================================================
echo                          ����� �������� - %VER% �������� ����
echo                                       *****[34m
echo.
echo.
echo                             ���� �������� ������� ��� 
echo               ������ ������ ��� ������ ������� ����� ���� ����� ����
echo.
::���� ������ ��� ������� ������ ����

echo              [%clear_heb%] ����� ����� ������ ���� ������ [0] ��� !���
echo                   ------------------------------------------
echo                   [%cm_heb%] ������ ����� ��� ������ [1] ��� 
echo              [%abc_heb%] '� '�� ������� ������� ������ ������ [2] ���
echo         [%sing_heb%] ��� �� ���� "�������" ��� ������ ����� ������ [3] ��� 
echo             [%fixed_heb%] ���� ��� ������� ������ ������� ������ [4] ���
echo                                ������ ����� [5] ���
::����� ������ ������
choice /c 012345>nul
::�� ���� 5 �����
if errorlevel 6 goto :final
::�� ���� 4 ����� ����� �� �����
::����� ����� ���� ��� ������ ������
if errorlevel 5 if "%fixed_heb%"=="���� ��" (
set fixed_heb=����
goto :options
)else (
set "fixed_heb=���� ��"
goto :options
)
::�� ���� 3 ����� ����� �� �����
::����� ����� ���� ��� ������ ������
if errorlevel 4 if "%sing_heb%"=="���� ��" (
set sing_heb=����
set "s=\�������"
goto :options
)else (
set "sing_heb=���� ��"
set s=
goto :options
)
if errorlevel 3 if "%abc_heb%"=="���� ��" (
set abc_heb=����
goto :options
) else (
set "abc_heb=���� ��"
goto :options
)
if errorlevel 2 if %c_or_m%==move (
set c_or_m=xcopy
set par=/y
set msg=�������
set cm_heb=�����
goto :options
) else (
set c_or_m=move
set par=
set msg=�������
set cm_heb=�����
goto :options
)
if errorlevel 1 if "%clear_heb%"=="����" (
set "clear_heb=���� ��"
goto :options
) else (
set "clear_heb=����"
goto :options
)
::��� ����� ����� ������� ������
:final
cls
echo.
echo                                    __   __   __
echo                                    \ \  \ \  \ \
echo                                     \ \  \ \  \ \
echo                                     / /  / /  / /
echo                                    /_/  /_/  /_/
echo ================================================================================
echo                           ����� �������� - %VER% �������� ����
echo                                        *****
echo.
echo.
::���� ����� �� ������ ����� ������ �����
if "%clear_heb%"=="����" (
echo               ������ ���� ���� ���� ������ ���� �� ����� ����� !���
echo                 -------------------------------------------------
)
::���� ��� ����� �� ����� ����� �� �����
echo               �������� �� ---%cm_heb%--- ��� ����� �������� ��� !�� ���

::����� ����� �� ����� - ���� �����
::echo                                       �������
::echo                                    "%p_finish%" 
::echo                                       ������ ��
::echo                                     "%h_finish%"

::���� ������ ������ �� ��� ������
if "%abc_heb%" == "����" echo               '� '�� ��� ������ ������� ������ ������ �� ����� �����
if "%sing_heb%" == "����" echo                "�������" ��� ������ ����� ������ ��� ������ �� ���� 
if "%fixed_heb%" == "����" echo                     ���� ������ ��� ������� ����� �� ������
echo.
if "%fixed_heb%" == "����" if "%abc_heb%" == "����" echo           !��� ������� ����� ����� ����� '� '� ��� ������� ������ �� ���



echo.
echo.
echo                  [2] ��� ����� ������ [1] ��� ������ ����� ������ 
choice /c 12>nul
if errorlevel 2 goto :mesader-singels
if errorlevel 1 goto :intro


:intro
::����� ����� ����� ������
::�� ����� �� �� ��� ������
cls
if "%clear_heb%"=="����" (
for /r %%i in (*) do (
cls
echo.
echo                           ...������ ���� �� ����� ����
set "file=%%~ni"
set "ext=%%~xi"
call :clear-func
)
)
goto :preparing

:clear-func
::�������� ����� ����� �� ���� ������
set "new_filename=%file:_= %"
set "new_filename=%new_filename: -���� ������=%"
set "new_filename=%new_filename: - ���� �����=%"
set "new_filename=%new_filename: -���� ������=%"
set "new_filename=%new_filename:-����� �������=%"
set "new_filename=%new_filename: - ����� �������=%"
set "new_filename=%new_filename: - ����=%"
set "new_filename=%new_filename: ������ ��� ���=%"
set "new_filename=%new_filename: - ���� ������=%"
ren "%file%%ext%" "%new_filename%%ext%"
exit /b


:preparing
cls
echo.
echo                                    ...����

::����� ������ �� ������ - ��� �����
set cm_heb=
set /a d=1

:start
::����� ���� ������ ������ ���� �����
for /f "usebackq tokens=1,2 delims=,"  %%i in (singer-list2.csv) do (
set a=%%i
set c=%%j
call :sort-func
)
goto :finish

:sort-func
::����� �� ���� ������
set a=*%a: =?%*.*

::����� ����� ����� ���� ������ �������
set/a en=%d%00/ab
if not "%en%"=="%enb%" cls & echo. & echo                                    ...���� & echo. & echo. & echo. & echo                               ...������ ������ %en%
set/a enb=%d%00/ab

::���� �� ������ ����� ������ ��� � �
if "%abc_heb%"=="����" set w=%c:~0,1%\

::����� ���� ��� �� ���� ������ - ����� ������� ������
set b="%h%\%w%%c%%s%"

::����� ����� ��� ����� ���� �����
set xx=v
set ss=z
for /r %%c in ("%a%") do if exist %%c set ss=ss
if "%fixed_heb%"=="����" if not exist "%h%\%w%%c%" set ss==z
if %ss%==ss md %b%

::����� ����� ���� ���� ���� ����� �� ����� ��
if %c_or_m%==del set b= & set par=/q

::����� �������� ����� ���� �����
for /r %%d in ("%a%") do if exist %%i set xx=xx
if %xx%==xx for /r %%e in (%a%) do %c_or_m% %par% "%%e" %b%>>�����

::���� ����� ��� ����� ����� ��������
set/a d=d+1

::����� ��������� ����� ������ ����
exit /b


:finish
cls
echo.
echo.
echo                                 %VER% �������� ����
echo                                       *****
echo.
echo.
if %c_or_m%==del echo                                   !����� ������ & echo. & del ����� & goto pause
if exist ����� (echo                              :%msg% ������ ���� & find /c "1" �����
) else (
echo                                   !��� ���� ��
)
if not exist ����� set c_or_m=xxx
if exist ����� del �����
echo.
if %c_or_m%==xcopy echo. & echo                 [2] ��� ��� �������� ������ �� ����� ������� ��� �� & echo                         [1] ��� ���� ����� ������� ��� �� & echo. & echo               !������ ����� ���� ����� ���� �� ����� ����� �� !������ & choice /c 12>nul & if errorlevel 2 set c_or_m=del & goto preparing & if errorlevel 1 goto :pause
:pause
echo.
echo                         !��� ������ ����� ����� ��� �� ���
pause>nul
cls
goto :mesader-singels

::�����: nh.local11@gmail.com



:creat-cvs

@echo off & pushd %~dp0
powershell -noprofile -c "$f=[io.file]::ReadAllText('%~f0') -split ':bat2file\:.*';iex ($f[1]);X 1;"
exit /b

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

:bat2file: singer-list2.csv~
::O/bZg00000K@)o?00000EC2ui000000|5a50RR9100000O#lD[0RRIPOCJCL00000000tIGGHJ8b7]j8WpXWSX?+WkE[N|c0C+f7eC!EJA6tNcjR6p{]Z+=;1T;m,
::U=2iqQb+J2M9L5M[BI5lGFeRLmg3Up){yA20s{auLU3nh0Ph5=K)$CaARp65NGi}_M[g]u=U.K7-q2z]U;apkJ|/Li&t#LM)IHNxwVejzPO.|KO-2iW&VpV{OIK=B
::OZ2LJLcjn1^dvJ-0EhspqzZ^/VeXlzt-brtu[z81KR.Vb_?/$Y7ZZ.pKe^pnWKF;|gtG$Z(m4hL;vMPkxE.&ox@Kd2$YZ5~B-([t{mqhsb27lA_r=Fe?n[e2it#oP
::SjShS6{VCKcMq5Rhc3PMM_N8jk)[O3]Q+tA?6d@Xz!H4BZ]EC{NbN7eGUPeJQaqmT&U6J@F!JOT09Bv[QfcI;n#eD~2&ke{=x,H9NFfFR$,v4VSOR7-sUTRPH,jQV
::5-T6q)Iq&mKrwmJdKr2226^c]MI]+!Q2pm8Ic/p[#-3Ml,&S@/lc6Xk.3$o_p&9ex8yl~4TWvn9Or~UV?kdRb8{OE_,][,vOdD{t(pG2.VVed/Q[otp[{PfyS!5)g
::.14k&a=2=~nb6rs?}_@[|26gTOi=/^iy2FtOqU?9bJ(Swm&;q~XYZ(/rzVlw^KF0HR=MFmpS.j~HH@MhPHLq#r[M1!id#f0K)It}l(D7aB2,EwqK^OgElHeet2So|
::uNQqTp=_=yLEz/$;Ggl=hby3vIF6kuhnVItNz6&Q.oXLB8;CoWq{+$$VF&/1QY+.lm{!N{0!D8&tMhKGR@!)$=YR},&6))v0hV_G04i}(q)Q)oH30cYWq4/-1X77H
::$h./7j&=E4SR;ex1){AfdPQY#+7_[/7+|zEr=T)~,L.{{Lk6-y$0By_90.rZy)N=Td[qB#$rZR.qS!A1Vv54[U$asFoefM[A=$7aDAae^RlcySA^ydz!8-Mml]Jkl
::PvTKh3|^qAGG#{SKbP]V5u-r+l?y0.UQ/;tAl3QEKCgH.TYK/[(9[ROOsPcP1v/DTJ=9KmLpwJv.sCfScXDxi@gH@qjK{LqN[HAbNGZea(HuxjS.CRu7Fe{!UFZ!J
::R;pgrgcNtdXq7+8MHMZXP,UZ4o0S3VQY-BJK(W+X|Jh=V^MjfS+;]s3,^Xi~@RzeQK-Cf;oC}yHnlX#]l,q/Le#g&YDBZGX+82&Dn$-Z5i0]u^w@$E^E;8?E?3hmh
::dR585L7zi~@Xt[anfHj-;uE!X+UYH=@/zwiA)O_iZ|ZogfEnY[)4SzMDNJlrEl|jSjPk5cxzt|)]2.$&HrO~2t0/_Y2p+[^{5?vKQr.|eefvElgM3CeDz3~fEd@|g
::XWR,y9QrdzXX)-diLY]h2X8^pU6Dg7;}Fo{m|g4_KBX5BH8598=(!g6MtB_GS)gwyUXU|$sNP10fVej~zZGg+oKN{gWo]!?G7rgf,rqz5USvfwz;tZk-k0^A+3{zf
::OV&_zS$~3#zK8S@4^b[e$QJb+/LvBktWUU[xG~E=_WYgOUk.G]N$^4lf4pnwjey$0[U36/ipq{MK61-|V0d&10!;0Rg70CKAsYvhfdrWICegvtcvj1{z/B|WrLG},
::Y?-VlmClyERDk]yuA;r4?zmC6iFu}b2-;ur_Lld1Ve1vCXL[,dxV_#mhO?D4vo}).&bL34^AsTrOSIiQdj!U(a&R{cjqb4ZZbIvSzx_^UpQ8EJGqo7{{W5G3^BYuM
::=d~Pt3$e0Yf|#4wZIfO1!]&9Vkl1i,v{2oWnFVBSUsSTN)lOczlc7+M$-rH#f0AR(T}bS//tjL,eaR63.,Rt!uyajAjE(ryEF@o;1o^rW.7pNA4WLLvs9NdE7IJUa
::^Td{oQbOQmx!;TpUrboh.XbLHccTcuNpk!=h}aRNnoy5-MGRGc0b22l5oWxXH2}AuvN$V1-Ij6l2@XnV;8.rl(u8M}.$6a2&qHpeaNKn(c-Z4n$c1)8w-09XTD4r9
::#_AZ3;I.n7SI,nFEmS,Q^++Niqu4cs0JSq+wCH4ewjGWBb#n=G!IYLJ)k&nfAr|3{;Ds-2;MrAcB_]jsKm=a+5IR2n^kTkoc8D7+dMn)!tOZnRb}1M_yuc|JUvt~L
::1btj(fpmR!#or=6J6S~Npn8pj.!7G1y0@F(#drO+lwoYe&kd@+Q0siU#13hhaL)xXD,7G}!l2bemTNs^RO3Z-st$,NSwcx?yB$lDmk$A|KQbMTi{di1fcHozo)&~/
::tFXn}ExHKww|&2WPYG-ppF}@NJC)8cvo^s#wD?c/hLZn9g1ZiBgvGSMA&6p/E@defKgff$tTNW![mReIu(4NewJ~CA6^Lh=ztR07/F9Bookn$yS0R9=pyt~212;RJ
::ihRhPGSAAhc!p_+6}1bwx|)-cCx!2D1?^/#HYsU?_k;u.hwdR#x)asuXq86CNdIc@Q3}D1-br~GR]0}xgl;(C-083b6Cq&e^8xfQEea]l?9nO8{W6E!Axh~@@,Z^7
::4R6bXe&?2DIeshm{D&OemB98d4oRJS/a+~r+E-g44nn;n8Mm_YF4o;^sN3d+wQS]w[4n$Wuzv&?g{RnjMKDFwo-OK&[EFyCE;O^on@|i)hj3NmdjlKwoxlmYRz8{R
::BUsHTiY)M!(djrXm{8_~6,PY,0z@}(S_J?gI[FrK0mG|B9vRpP8l/{dgqOu[arM@tQ!c(d[#CG+U5r-Z9+JxTk.8KX@;bpDaLh9QYzouaRIdEUIv#mRyOSRT[ai$o
::Y)@UQc#7Yr?1ZES@qA)}tj,t]Va82tBh[eC;Gm2aeK2+sP5.~6G?}?MAtxm~k-w3q4u$S[g&?{;uXqS5LW0RJ13OnEbdOGc/ol_0K/~.viUtf!rSGSg]T(sb$L#w,
::GYeYSx?L7q]G7s^ccvTw=;)4,Y|j/;R,9c#bokJ&uWAUVHUE4w/1(O{TmPARq9AqR-1?EI#^(srt{4ZU(fY-|@fh(3M6Q.[W0a{q|5olZsS4$bqq?iha,mqbU5vot
::8D1=T+]gt55Smo9h&4apj}5d(,BM|;T|jNq.ep_@[JD.DL|5ou[[Cu1u&h03S(tw;p?JEl3)H+u{BH)HyEe4743u^u?B749GV=./GCVaqgJq+,;EVWF;s]P$tL@|M
::VMcEVrNfq2;[#rKNh==D|2]9(;P3I;,=W#qhQ1MQcFVHoQD9^T(-k.jzR~coqq8Mi=wPf3M)wn$_mVc6fw?C72!fA-/.qp80!YG@,m=;jB~M4_AsCDnvDw|hNHbrq
::kZ57-438odxAZ?LCR1v!jziGBdY_}elm
:bat2file: end




