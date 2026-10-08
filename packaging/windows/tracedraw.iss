; Inno Setup script for TraceDraw. Built by CI on windows-latest.
; Variables: /DVersion=x.y.z /DSrcDir=<dir with tracedraw.exe and tracedraw-cli.exe>

#ifndef Version
  #define Version "0.0.0"
#endif
#ifndef SrcDir
  #define SrcDir "..\..\target\release"
#endif

[Setup]
AppId={{7E6E3B5C-4C1E-4F7B-9D7A-2B9D5C1A0E11}
AppName=TraceDraw
AppVersion={#Version}
AppPublisher=TraceDraw contributors
AppPublisherURL=https://github.com/igtoth/tracedraw
DefaultDirName={autopf}\TraceDraw
DefaultGroupName=TraceDraw
UninstallDisplayIcon={app}\tracedraw.exe
OutputDir=..\..\dist
OutputBaseFilename=tracedraw-{#Version}-windows-x86_64-setup
SetupIconFile=..\..\assets\icon\tracedraw.ico
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequiredOverridesAllowed=dialog
ChangesAssociations=yes
ChangesEnvironment=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "addtopath"; Description: "Add tracedraw-cli to PATH"; GroupDescription: "Command line:"

[Files]
Source: "{#SrcDir}\tracedraw.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SrcDir}\tracedraw-cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\TraceDraw"; Filename: "{app}\tracedraw.exe"
Name: "{group}\Uninstall TraceDraw"; Filename: "{uninstallexe}"
Name: "{autodesktop}\TraceDraw"; Filename: "{app}\tracedraw.exe"; Tasks: desktopicon

[Registry]
; .tdraw file association
Root: HKA; Subkey: "Software\Classes\.tdraw"; ValueType: string; ValueName: ""; ValueData: "TraceDraw.Document"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\TraceDraw.Document"; ValueType: string; ValueName: ""; ValueData: "TraceDraw Document"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\TraceDraw.Document\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\tracedraw.exe,0"
Root: HKA; Subkey: "Software\Classes\TraceDraw.Document\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\tracedraw.exe"" ""%1"""
; "Open with TraceDraw" for .cdr, without stealing the default
Root: HKA; Subkey: "Software\Classes\.cdr\OpenWithProgids"; ValueType: string; ValueName: "TraceDraw.Document"; ValueData: ""; Flags: uninsdeletevalue
; PATH
Root: HKA; Subkey: "{code:PathRegKey}"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addtopath; Check: NeedsAddPath('{app}')

[Run]
Filename: "{app}\tracedraw.exe"; Description: "{cm:LaunchProgram,TraceDraw}"; Flags: nowait postinstall skipifsilent shellexec runasoriginaluser

[Code]
function PathRegKey(Param: string): string;
begin
  if IsAdminInstallMode then
    Result := 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment'
  else
    Result := 'Environment';
end;

function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
  Root: Integer;
begin
  if IsAdminInstallMode then Root := HKLM else Root := HKCU;
  if not RegQueryStringValue(Root, PathRegKey(''), 'Path', OrigPath) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Uppercase(ExpandConstant(Param)) + ';', ';' + Uppercase(OrigPath) + ';') = 0;
end;
