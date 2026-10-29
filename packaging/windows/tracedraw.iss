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
AppPublisher=Ighor Toth
AppCopyright=Copyright (c) 2026 Ighor Toth and the TraceDraw contributors
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
; Ask which language the wizard runs in (the system's is selected).
ShowLanguageDialog=yes

[Languages]
; The wizard's own language. These are the translations that ship with
; Inno Setup 6; the editor itself has more interface languages than this,
; chosen in Options > Global > Language. The six the editor has but Inno
; Setup does not ship (Arabic, Bengali, Hindi, Indonesian, Simplified and
; Traditional Chinese) need their .isl committed here and are a follow-up.
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "czech"; MessagesFile: "compiler:Languages\Czech.isl"
Name: "dutch"; MessagesFile: "compiler:Languages\Dutch.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
Name: "polish"; MessagesFile: "compiler:Languages\Polish.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "turkish"; MessagesFile: "compiler:Languages\Turkish.isl"

[CustomMessages]
english.AddToPath=Add tracedraw-cli to PATH
english.CommandLine=Command line:
brazilianportuguese.AddToPath=Adicionar tracedraw-cli ao PATH
brazilianportuguese.CommandLine=Linha de comando:
czech.AddToPath=Přidat tracedraw-cli do proměnné PATH
czech.CommandLine=Příkazový řádek:
dutch.AddToPath=tracedraw-cli aan PATH toevoegen
dutch.CommandLine=Opdrachtregel:
french.AddToPath=Ajouter tracedraw-cli au PATH
french.CommandLine=Ligne de commande :
german.AddToPath=tracedraw-cli zum PATH hinzufügen
german.CommandLine=Befehlszeile:
italian.AddToPath=Aggiungi tracedraw-cli al PATH
italian.CommandLine=Riga di comando:
japanese.AddToPath=tracedraw-cli を PATH に追加する
japanese.CommandLine=コマンドライン:
polish.AddToPath=Dodaj tracedraw-cli do zmiennej PATH
polish.CommandLine=Wiersz poleceń:
russian.AddToPath=Добавить tracedraw-cli в PATH
russian.CommandLine=Командная строка:
spanish.AddToPath=Añadir tracedraw-cli al PATH
spanish.CommandLine=Línea de comandos:
turkish.AddToPath=tracedraw-cli'yi PATH'e ekle
turkish.CommandLine=Komut satırı:

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "addtopath"; Description: "{cm:AddToPath}"; GroupDescription: "{cm:CommandLine}"

[Files]
Source: "{#SrcDir}\tracedraw.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SrcDir}\tracedraw-cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\TraceDraw"; Filename: "{app}\tracedraw.exe"
Name: "{group}\{cm:UninstallProgram,TraceDraw}"; Filename: "{uninstallexe}"
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
