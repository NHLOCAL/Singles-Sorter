; Compiled by .github/workflows/release-v15.yml from an already-accepted desktop package.
; Required defines: MyAppVersion, MyAppId, PackageDir, OutputDir, RepoRoot.
#define MyAppName "מסדר הסינגלים"
#define MyAppPublisher "nhlocal"
#define MyAppURL "https://singles-sorter.ze-kal.top"
#define MyAppExeName "singles-sorter-desktop.exe"

[Setup]
; The AppId is shared with version 14, so this installer upgrades it in place.
AppId={{{#MyAppId}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\Singles Sorter
DisableProgramGroupPage=yes
LicenseFile={#RepoRoot}\LICENSE
PrivilegesRequired=lowest
OutputDir={#OutputDir}
OutputBaseFilename=Singles-Sorter-Installer-{#MyAppVersion}
SetupIconFile={#RepoRoot}\native\apps\singles-ui\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
SolidCompression=yes
Compression=lzma2/ultra64
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "hebrew"; MessagesFile: "compiler:Languages\Hebrew.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[InstallDelete]
; Version 14 was a Python application. Remove only its known program files.
Type: files; Name: "{app}\main.exe"
Type: filesandordirs; Name: "{app}\app"
Type: filesandordirs; Name: "{app}\assets"
Type: filesandordirs; Name: "{app}\models"

[Files]
Source: "{#PackageDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
const
  WebView2Client = 'Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';
  VcRuntimeKey = 'SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64';

function HasWebView2: Boolean;
var
  Version: String;
begin
  Result :=
    (RegQueryStringValue(HKLM32, 'SOFTWARE\' + WebView2Client, 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0')) or
    (RegQueryStringValue(HKCU, 'Software\' + WebView2Client, 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0'));
end;

function HasVcRuntime: Boolean;
var
  Installed: Cardinal;
begin
  Result := RegQueryDWordValue(HKLM64, VcRuntimeKey, 'Installed', Installed) and (Installed = 1);
end;

function InitializeSetup: Boolean;
var
  Missing: String;
begin
  Result := True;
  Missing := '';
  if not HasWebView2 then
    Missing := Missing + #13#10 + 'Microsoft Edge WebView2 Runtime';
  if not HasVcRuntime then
    Missing := Missing + #13#10 + 'Microsoft Visual C++ Redistributable x64';
  if (Missing <> '') and not WizardSilent then
    Result := MsgBox('התוכנה דורשת רכיבי Microsoft שלא נמצאו במחשב:' + Missing + #13#10#13#10 + 'אפשר להתקין כעת ולהוריד אותם מאתר Microsoft לפני ההפעלה. להמשיך?', mbConfirmation, MB_YESNO) = IDYES;
end;
