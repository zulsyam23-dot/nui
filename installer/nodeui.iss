#ifndef AppVersion
  #define AppVersion "0.2.0"
#endif
#ifndef AppURL
  #define AppURL "https://github.com/zulsyam23-dot/nui"
#endif

#define AppName "NodeUI"
#define AppPublisher "NodeUI"
#define AppExeName "nui.exe"

[Setup]
AppId={{98A84409-2BE8-46D0-B2CC-0E6034A145FD}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL={#AppURL}
DefaultDirName={autopf}\NodeUI
DefaultGroupName=NodeUI
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputDir=..\release
OutputBaseFilename=NodeUI-{#AppVersion}-setup
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\icon.ico
UninstallDisplayName={#AppName} {#AppVersion}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
ChangesEnvironment=yes
MinVersion=10.0

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "..\target\release\nui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\icon.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\examples\*"; DestDir: "{app}\examples"; Flags: recursesubdirs createallsubdirs ignoreversion

[Icons]
Name: "{group}\NodeUI Examples"; Filename: "{app}\examples"
Name: "{group}\NodeUI README"; Filename: "{app}\README.md"
Name: "{group}\Uninstall NodeUI"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\README.md"; Description: "Buka dokumentasi NodeUI"; Flags: postinstall shellexec skipifsilent unchecked

[Code]
const
  EnvironmentKey = 'Environment';

function NeedsAddPath(Param: string): Boolean;
var
  PathValue: string;
begin
  if not RegQueryStringValue(HKCU, EnvironmentKey, 'Path', PathValue) then
  begin
    Result := True;
    Exit;
  end;

  Result := Pos(
    ';' + Uppercase(Param) + ';',
    ';' + Uppercase(PathValue) + ';'
  ) = 0;
end;

procedure AddToUserPath;
var
  PathValue: string;
begin
  if not NeedsAddPath(ExpandConstant('{app}')) then
    Exit;

  if RegQueryStringValue(HKCU, EnvironmentKey, 'Path', PathValue) and (PathValue <> '') then
  begin
    if PathValue[Length(PathValue)] <> ';' then
      PathValue := PathValue + ';';
  end
  else
    PathValue := '';

  PathValue := PathValue + ExpandConstant('{app}');
  RegWriteExpandStringValue(HKCU, EnvironmentKey, 'Path', PathValue);
end;

procedure RemoveFromUserPath;
var
  PathValue: string;
  AppDir: string;
begin
  if not RegQueryStringValue(HKCU, EnvironmentKey, 'Path', PathValue) then
    Exit;

  AppDir := ExpandConstant('{app}');
  PathValue := ';' + PathValue + ';';
  StringChangeEx(PathValue, ';' + AppDir + ';', ';', False);

  if (Length(PathValue) > 0) and (PathValue[1] = ';') then
    Delete(PathValue, 1, 1);
  if (Length(PathValue) > 0) and (PathValue[Length(PathValue)] = ';') then
    Delete(PathValue, Length(PathValue), 1);

  RegWriteExpandStringValue(HKCU, EnvironmentKey, 'Path', PathValue);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    AddToUserPath;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromUserPath;
end;
