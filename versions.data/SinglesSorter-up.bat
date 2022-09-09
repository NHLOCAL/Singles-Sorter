::���� ������� ��� ���� ����� ������ ������� ��� �����
::������� ����� ������ ����� ���� �� ����� ��� ������ ������ ��
::�����: nh.local11@gmail.com

@echo oFF
::������ �� ���, ���, ����� ����� �����
::���� ���� ������ ������
chcp 1255>nul
set VER=8.2
title %VER% ���� ��������
MODE CON COLS=80 lines=27
color f1

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
echo                                ������� ���� ������ *
echo                            ���� ������� ����� ����� *
echo                             ������ ����� �� �� ����� *
echo                        ������ ���� ����� ������ ����� ����� *
echo                                ������ ����� ����� *
echo.                          
echo                   ����� ���� �� ������� ������ ������ �������
echo                ���� ����� ����� _ ���� �� ����� ���� �� :������
echo              ����� ������ "���� ������" ,"����� ����" ��� �������
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
if errorlevel 2 (start https://github.com/NHLOCAL/Singles-Sorter/releases/download/v8.2/help-singles-sorter.pdf
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
set "msg=�������"
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
if exist ����� (echo                                 :%msg% ������ ���� & find /c "1" �����
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
::O/bZg00000@g{^]00000EC2ui000000|5a50RR9100000OaK4@0RRIPrXc^T00000001dfT[WAub7]j8WpXWSX?+WgV{?+[aR@1$nhK]NTY!L#0uW,v002}3G.3c@
::4Mc+cN4Ky;$_AMN{QE{SS4^^?#ih+r=,R#B2moee/Lgkdo+lYwZB7eLF}/ZNpm,F]R^xwAb$|Y9R[,f1i=YW+oP0V@#M#N=I@6-E$Z8T39-L7{GMjn4l(gB#,QGNx
::sk(z@(mnyWzyJRCK#&|c2ms7R3~YX3_@lgEQbmUm-X4Ig|ND~Id}U0rmvDdn&FP_lYw}Q}oDoBR$LNzP+iM3T8-w)LH+GVMiQdx|I#IrE{8+z)M+{g!03qbNev6mZ
::Z]Eg9!p.zmxJ51Pl)Mm))^7_o{ATZuBRo2hnH1+yOGnAFe,V)IN$Bjv0e@?/yuS#}$n!/|;UVoBM,yfi]5hYKRi5&uWyCa&-80p.pFd{eZ=7Xx[bUm/Got~Pd{|4W
::^@71j{MeaP3NZaS1V/#{CeBVTA5Y/ZkASQ|WOxFI./5=&#!hdX!gtG}XsDZn1QAUC;d=jpPa;c+ywIkO{N;U.&U9D+h~)rLQ{z,2#7bS6fP/0/9ixPGnrB5VIU(V+
::7N@]wNJzM/8Ik0~?EW8V)H_s0kj6f2?NkmI1mTF4O8rck0GxA_i5i#G8B|}wsWQ^~NTfS-[=P]F[FHK,S)]&4#.S)1)qW]s9L@h1teB83pb;)}yD[-fZO$lF2aNLU
::Jd.wmrKT$L7SQypStWOk_|G&P9nO=TK,X96Bw#YRc,7@WZUn68x+Mt,=ZZOkoaJAI@kra&jWUAXteCP]Smk#Ceii2;T|+srq#2Q^07umP-FNr0o[sHm1sZfpCW=,Z
::pq;ofpgM{ojUMYTx~QUqhTkaa,rB{Ox}aElpuI,?sb{9v5=+Na$ZuYNNy3)rqbSHbp0uD5FIi(HjRC)9Mp18$Fa3qJsaK$2./Gv9qGoFNkgkj)!T!^Kz.c,MN0o6!
::=@c3b6^5Xf#jAGQl4[6sAOm]@;phCD;}dox/);6ji^]6CO1[yk0,)5kG=2MXJ.3EfFLc~rXPmT!ar(&s-t)KbWloih7=4bWh2dM&hrQGOW/8.^t@yaa48^(wx7C1|
::cDg80ccO|ag-5Rb=9imN_G!+3Q0hD,ro)SkVuSWoJ(e?5,}Imfqo~)ASVF;|X9PGG4{0=)&sSJ?^L{t2KPR9V(Mt~}iDEUK$pH-hHMU~jB4YRkxU8h}=?pl!B}0Rz
::qkz~ql@yUv2].7b#7m.K$Sa?.WlJa];C)EYw1PloX-})o+HBH]8fFWVc&~8R5r9]l=^#Ii0#L?|2cr[j7ZC)d2Ig;)jn7hYqmz[W{8P+}X6RG?![=3pGa1pueV#hg
::h-NGG)=l;fXEN6DhY=Is_B[([q5xTEHAjqB6038.wd+!E(k@?lTQ/JIT7O(yCH(D=Wh[,jc-n^ecjn;2aapAqY[b~9=F2KmfSkwulD,LFMx-7]=j{1}EzXW]q$3lG
::9meIgP81jSGJfGn[U9yTioO}y_OAmZsk{]RbQj;qQ84-LoDWP4tyMddX+81GV4v{2)db9Occ39tb9VWVK3frxN}jfB]{U0k-|w?zU|LKRf9K/^p+g=.VN1sA6#hD1
::Tl[._yMR4-=UPW}n~v-;&hsVs,fSygQx814hfmll@Ml)0BjioKJV)x,ymi[C))}$U[vU7il;F$M]7L.o8-7+G+Q,g11wYAH-uVLcs^+;Y1MOoZF{XPd,zx9FaZ=7O
::@iek-G{]+aL#tshIxvTaeeksD+0@@o+~^@6z$Cl~VD6=;v{5EgPZCEIRA~Ouj)cZG;ue-cS8=z30Tbs7M$(C|!Y7ElJqq|BqrpJ18&5QBury#rtkL@3K7]sQ;}5Go
::VOa(9@QbIlS+B8WU3B1plIkZy3Uf7x|C]Ag)4m&)l4]+H.iU8sp9]TkG6q.CYTE=HKqYBLfv+n]g#s1Zb^VID,_Cjs#Xkf4hnYdf&g5tRRKNQrN-v?VaKj}+u=Lh+
::aazxw@2Jes_(iJ=]^HU)I_E]w2[c95iWmAiE!T9aRZDHfzI0OwbD]|WCerN!(|wx}jmw?,N8|IqT,?ZL((34Yc[+u+pKG77r?4ofD(iaTU3CJgwmbB5A?Cb+&R0X2
::RYIzGRDC3UcEsKyJUfI$bI9w)jFWtmv~,s.d_lDFuX$[{B}7octd@Z!DY|F|+|XYBA.6HjHVP&E2af_8P!|e$/hgG[Y_{B0vd2gL0h5PE4z+$O8J7TYHC?PTgsaVh
::VqeVb(K0.&qco2^bxOa4kk$@;EcIYrFaFiO;;7MJk8mmZ15~(aY6)(75Zv.L+TT~vOUhYgtX;w-cDoR=^XxIUV!/wGvlY+s{khxG^aaRRJHjhNz.D$7,i]b+h;;VC
::.C2bXnbZ4NFUw.K!aLvIDzOu^MXaFU+QyHb,xsLHR3H$p#o)D9r[N)4h/O6C3?oU(,gMoHXWU|8JTdt-tx~)897nXSNlc[FChR=uVoMQFpV$0KWlqT4t+rCEm!1OM
::-!@)sSXyjn]yKTUyz_C&NBe/6J{czw9+funG}Ls|3OXd)@=E0xV#ICzM]SEr.^4^uBeW.j=)^j}uP(XI@h)M0Qg@$Sr[$zx2);c4BJ3@Xlm]|Z#rKC75;lRCNh7~Z
::;^Xkh5JmdpT,UAw9wwAIZv?w|L{Qe]jTUd0PmdZCGk|pR&41Q!kdf@{KS+~qP2SxqW~$Pg^O9J|.!&ve#iOX98xoa,/#y@Ui|pA(kL^Xhdr6hQS)hIV)sGGM0{HqE
::D@5^+84|?B){-50R^@E!IaV&zJi)Bg,hZyKim(NH[L.846OG3IYVHNHY9DM/g{J}5J=U;G$BxwE0i$t8Y7zJ+ybIf!jiEkz7Yz6-)104Lz$12RWGZ+_yIek;XF6fp
::@w7kDKkik1;FuzkK+kl)27rl/7,eW]6tzwKH4DVY5O.pRch~/a{{pL_5,G0PVow]RzB~CFW{V]I/Mo}CsMy,WLbt[#7eHohsy8Q[.}JSjj!9Mcxf;phDgE^_6i_MH
::N~pG&cix3ug(vqmrf@jAg]zEa1HHNcYSIn.HRs(Lf)z04auWOz9-|B1]+BWo$Q{Td5?Ue47x2C]!TXMjU)y8|TiY~BJ-l_vl6l0u(2=9~=,[T8o_X4RA3?X1+C/!#
::j5bv05};s,PMM5;p^V_52le-cN}=Yi=,vC?9LVc4P3IYwH/s+V#Z}_[,X9[vqI[1)ttHj{ZxnAQWmQ00-FuZ;CW]QF?k}X~QxuVesBqGv843;Iy5Pkz3x|7uilVt$
::/ex5cZOeHSp}1lAqcRvJe)4tk&cXSq/GZnZSl;9.g3mWy8R|4vw!Qk$Xw0u(;0=B)fI8VIed,/d4EFJ@l0z&{{cWA^ikRLo[(m{)6z=~=F}eR2ndBi~^FclcV=x;?
::f!7Kh1~1=+SR=RJ.p#SeGLw6+RI4L]inR@SFvr$sA=/dy9tBp.V+.mORT77;r~&y)/(?$z/dteB|0is2-5^z{9@^0-Au.huK3_J=9IC{)879CXX-{^!9kaaT0Kx
:bat2file: end

