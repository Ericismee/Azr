; Compile with ISCC /DPayloadDir=... /DOutputPath=... installer.iss
#ifndef PayloadDir
  #error PayloadDir must point to the binary-only staging folder.
#endif
#ifndef OutputPath
  #error OutputPath must point to the installer output folder.
#endif

[Setup]
AppId={{B35F3A75-BE5C-4A91-9962-EAC9CC889BC3}
AppName=Azr
AppVersion=0.1.1
AppPublisher=Ericismee
AppPublisherURL=https://github.com/Ericismee/Azr
DefaultDirName={localappdata}\Programs\Azr
DefaultGroupName=Azr
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.22000
OutputDir={#OutputPath}
OutputBaseFilename=Azr-Setup-0.1.1-x64
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
Name: "{autoprograms}\Azr"; Filename: "{app}\azr.exe"; WorkingDir: "{app}"; IconFilename: "{app}\azr.exe"; AppUserModelID: "Ericismee.Azr"
Name: "{autodesktop}\Azr"; Filename: "{app}\azr.exe"; WorkingDir: "{app}"; IconFilename: "{app}\azr.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\azr.exe"; Description: "Open Azr"; Flags: nowait postinstall skipifsilent
