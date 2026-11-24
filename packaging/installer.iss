; Compile with ISCC /DPayloadDir=... /DOutputPath=... installer.iss
; Add /DWindows10Preview for the Windows 10 22H2 compatibility preview.
#ifndef PayloadDir
  #error PayloadDir must point to the binary-only staging folder.
#endif

#define AzrVersion "0.1.9"
#ifdef Windows10Preview
  #define AzrMinVersion "10.0.19045"
  #define AzrSetupName "Azr-Setup-Windows10-x64"
#else
  #define AzrMinVersion "10.0.22000"
  #define AzrSetupName "Azr-Setup-Windows11-x64"
#endif
#ifndef OutputPath
  #error OutputPath must point to the installer output folder.
#endif

[Setup]
AppId={{B35F3A75-BE5C-4A91-9962-EAC9CC889BC3}
AppName=Azr
AppVersion={#AzrVersion}
#ifdef Windows10Preview
AppVerName=Azr {#AzrVersion} (Windows 10 Compatibility Preview)
#endif
AppPublisher=Ericismee
AppPublisherURL=https://github.com/Ericismee/Azr
DefaultDirName={localappdata}\Programs\Azr
DefaultGroupName=Azr
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion={#AzrMinVersion}
OutputDir={#OutputPath}
OutputBaseFilename={#AzrSetupName}
SetupIconFile=..\assets\azr.ico
UninstallDisplayIcon={app}\azr.exe
VersionInfoProductName=Azr
VersionInfoDescription=Azr Setup
LicenseFile={#PayloadDir}\LICENSE.txt
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
DisableWelcomePage=no
CloseApplications=yes
RestartApplications=no

[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "{#PayloadDir}\azr.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\azr-guard.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\vcruntime140.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\vcruntime140_1.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\THIRD-PARTY-LICENSES.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\fonts\*"; DestDir: "{app}\fonts"; Flags: ignoreversion
Source: "{#PayloadDir}\brand\azr-liquid-glass.png"; DestDir: "{app}\brand"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Azr"; Filename: "{app}\azr.exe"; Parameters: "--open-settings"; WorkingDir: "{app}"; IconFilename: "{app}\azr.exe"; AppUserModelID: "Ericismee.Azr"
Name: "{autodesktop}\Azr"; Filename: "{app}\azr.exe"; Parameters: "--open-settings"; WorkingDir: "{app}"; IconFilename: "{app}\azr.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\azr.exe"; Description: "Open Azr"; Flags: nowait postinstall skipifsilent
Filename: "{app}\azr.exe"; WorkingDir: "{app}"; Flags: nowait skipifnotsilent; Check: IsAzrUpdate

[Code]
function IsAzrUpdate: Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
  begin
    if CompareText(ParamStr(I), '/AZRUPDATE') = 0 then
    begin
      Result := True;
      Exit;
    end;
  end;
end;
