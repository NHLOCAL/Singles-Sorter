::���� ������� ��� ���� ����� ������ ������� ��� �����
::������� ����� ������ ����� ���� �� ����� ��� ������ ������ ��
::�����: nh.local11@gmail.com

@echo oFF
::������ �� ���, ���, ����� ����� �����
::���� ���� ������ ������
chcp 1255>nul
title %VER% ���� ��������
MODE CON COLS=80 lines=27
color f1
set VER=8.1

::����� �� ���� ���� ���� �-������� �� ������ �������
::������ ����� ������ ���� �����
::����� ���� ���� ����� ����� ���� �� ����
if exist "singer-list.csv" (
set csv-file=singer-list.csv
) else (
if exist "%appdata%\singles-sorter\singer-list.csv" (
set "csv-file=%appdata%\singles-sorter\singer-list.csv"
) else (
call :creat-cvs
set csv-file="%appdata%\singles-sorter\singer-list.csv"
)
)

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
type "%csv-file%" | find /c ",">"%temp%\num-singer.tmp"
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
if errorlevel 1 "%csv-file%" & call :call-num


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
for /f "usebackq tokens=1,2 delims=,"  %%i in (%csv-file%) do (
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
for /r %%d in ("%a%") do if exist %%d set xx=xx
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

if not exist "%appdata%\singles-sorter" md "%appdata%\singles-sorter"
move "singer-list.csv" "%appdata%\singles-sorter\singer-list.csv"
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

:bat2file: singer-list.csv~
::O/bZg00000R0/q900000EC2ui000000|5a50RR9100000OaK4@0RRIP+gJ&=00000001aeN{Ao/b7]j8WpXWSX?+WgV{?+[q/?q,01DL~TY!L#0T5FC002}3G.3c@
::4Mc+cNw.,{;p=wB{{15vD;;cb/@m~RbYuVm0{}BZfLLb$@,yxW-DI,s$M_tn0iQck;_sWWs=96ecdJmhy-zl.4(A$i)4j@gNgr)p2U4kHdfZd2t7mEtHsw.T-UC.g
::?eNE4.&tPj{ojCK007Vc8O;0^yTkk5k6fBa6J~&Q^gDA#,#Cu]a!KKs{5O.oOO{GHnJ_vB{DDcyRXL8G7q.snl.!xCw#^._UnE3DyXr)4Mj.Ex=Zr.?mjM]kEk5ZF
::)K-T=EZ}vN@f62jN1hJZczcV#vAJjca,9eRo|CkGJavC8[ZnA!oW$2I+9[QL4Ev41EAmfSGQ}YL&[w/UkUVGwm6dWy(NK])0_O5V_RBLUk_t3OB[u?zEMKxR$(y(G
::H$Y{h6LG-3)IL9)fF,&edNFyjCs^pqMS8|^a3tptN(T!R#BKQG,/Gx1lN?1G.lGvFsSKP58z8UrQzL$JP/F)b?Lz6JRt(9yt.FGyFipV8LT_[K!pcpwrZ!!X0=?oC
::QIRC19fhq)asqU4Ok8a5cIe9^4?$UarW!$bVUSkBQ@@8#Tqdg4rIiNPn{cYm?AfV=.D!!owITo_Z^=(L5ijFl6JqJK(sxsrb$8fI(=&4PCoIw=0}l0h;76MQ&c}uR
::+&.U#SDn3vp[-,dLTuq;/]KBXkF~&Nc~-Z.$]^#LoJhkFv8MP6UbUVq;^fWvNtW@;J0x)Fl=EiAm(M3xxD0TNe3J7u72ryq6|oR?O]pD+I2qV8E6&D(BK5fw(MP,~
::9V.!3mx4]E?b|5hx9Pg!6pSXjl-/lfmum{TmmziQdywK&@/H$_q_f7SQCu]FxXTruTBP;b05M17^&GQg|IRI+Rxw#?BPi2{-Eu=-&rXcjo54ESExHUiGN|~f8LlK?
::aTzk@[~6vq+s9kf=t^WPNUy4#CJ]fUSf[8U8|S[vl,YA^D/TRt@g=_Z_#rp#dPB2UKJEe}y4Q1cdoBX-sEo.n(Q61jbV[11{^X$6n?qP1^#,6;?Mrz#3v0|?VZ(&k
::V3f-Csj7}eO|a@m!A-G)?ys._#zd+jE;dlw8tpMXc(Sh_Q7torLfQ6Q1wj{RX@QMa+[Y0wHmJQV4tPoaWkBwj#1wTW2NkSl-G2s&hKU[CwCc|5445~l4CRWR0uKKi
::E[-lzY{?s7FD|1ISWLr~FWrEW@}S|[6|Ah}vjT^AJ&fB=ai&b?C&UPW0T~5ao]q;MfMl2}Fl)_Ok?Wswa9.zf;TsSw^[?0^{/B4GvvkUzoOm5QqmfNO;{J~/$fvA/
::Df4GbC=+e,8et$m.;s(o1n^kych(e-u,@VTekG]Bxgtxav(I/4])f_xf?3p3(Jw9gBh3PW.W?euT!wj8/cIt2c{9t[EGbz);neUB9V.BnG0P;37w;?A(=o8S9R}rv
::C?YCoF~9L7fS8U3O(]SH{pQRHRUSz[x{PE_5Dfgwz=KY5^bQ!9ueCRFVx#as)fC+ucBCQGW0sPZ{njfWltk];ELMz;xu{GWz[$4N6_qZUi$wxcui9r_hVa-P-~ruq
::TL$WRyZ5}JAa!i.E!+YPubv3u.FnQ@viXF+-V7qXJ96CT(vWPG#ZRAok;w-psdIO/Q?[Db^1D|B[9Np7S]G4aT7HwY-vt8ps{cRzf&Y@]7}b6CEc+~]agt{]Sqv6t
::8uWrGon;hXeVD]zzxLo9^MEcV^G@fmvdORlnLCLpZPv/lLk5WDit9g{k=L&tcJ4u]nOeVy0SosEH.Krk7Cs=#?mkNRDNO?BY&ouMf-YqkG|iPz]dSrFH+ebJ4GSyq
::aDQAO+WWZiETabpl3Ys?RM[6b{ojPd2p]{UB)aF--OB/2{aixJ7cs^=P{ShV0;6um4wR6/F4d]Oxi@TZO!tAtGyWXaKFbbbt}c&|U;vD-9vLrU)GAN0Wwu-}$!R_=
::v]On&@sG!FwA-79@8A+$5,,M,Azs$mZONi[PTFfM[T/Fpo)tx,K#]$~iH]1mZQKu@RwmEqage^lLKhHx)nE{?{oehJo^HtktBBip@XDGIv+d[44,7gTFYEcLmI-kJ
::ag|f_+fIq@0qrXi)LpyG87De7=/^{mpF4n/ZRYK+;ajE+)iU__O^xt1dlQNi^_RTVHH0PT6O{sEQW@v5/T$WAalpGmv!_br0hA{X9g2&WGH!t3d()d7Nmr]7#r~|X
::NPE}1qqJXrt4F]HK+gJpu[s|~ANg4$n?+k&e}t8C4[H1GvX({u1JN#jj(bdBwWOAn##$R,?$iX-g0EnEBNpNjG~N4-@,H-Q#20=},A.nE05+xhrA^C{uILqau(rhI
::l|kX2;7O,nExLQ_D.&1BU;3,ZL,H&4hu!+itwM,8Ax69FmAY7pjrjO1YQ|Cb!Ty]H{E~JohG@YST(vk_E5{S;fRgqhe3LyMDADQ+r&w~PhnXRA?DaGY@[bMX3wma6
::8;siUD@$8&0RM[H^M/5IB7eb@3=c}ZiW)ihd=VM+GkCGL9~o3,[nG8R]1pXBa76cH0o}I0@dqaa0={pUN9uT&YHt]e3IS0cSA@QHl~w-0hWO{uLU|Q&rmBTkGXo.C
::b(5Mnwky-k91sG]Z[X/gk|GN&rAOo5#L=kc{0,4?Z}McQCTOzyzYw;/R,(kpd778$JcpX]h3{a5=;+;?=!{pUv]Y!I)PC@s{&2VjZxgoiA@teOBrQL4LjXS@er7_U
::FGg1kI8lfD(vFH8nPV,T$rA,,!g[_8g8V}l@L!k(FPg3Y+!Yc=$$hLz8J.7P^gumPzq.|n1dQQ3L=]$TpO;udQX|_s=7Kn1EgD5rrhQhoA,cKM.{tY_KJyIg)BJQZ
::{kT]3iQDZNhvT?[GypoB#n6??uc~O_vRW&XLg;ki.)~nq{|r{WCamQD+t+v/Jb3drwH8;S!M8KkP].B(i,AUeGJwq2+ou@t.tlVnKAWl^-(&McoPK+b3h2QI5~S^5
::orfX!;qfDwXVF}NnQz~c26~kN,2E3uHR[i,!PRuQphta$kIdF|dl~Z-;Szsg3n;}k3wWQI/ECtWH+-8JzLuTR,Dgj?kzQtA22-ry]rv2W=n#$@SI|w~D~8y9MjNX1
::$#6ety(},,kV{|jgZ&E9Od/m4XUty&2mbpFyg4n+pGkv}aY@,Xx&o!J)2mcRYn8wh8/snkSrvkoECr.G1|$15D~TX=dJsqwrozjMV;;b@^=pn6T{Q0f6]uD]@h2$L
::!p/LJQ,lD[Ms-x+e)D?7_qWMS$ftEic|W{Qjm~?z,ZF&EOQu7e(Md};
:bat2file: end

