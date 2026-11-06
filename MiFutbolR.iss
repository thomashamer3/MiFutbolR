; Instalador de MiFutbolR para Windows (Inno Setup 6)
; Compilar con: "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" MiFutbolR.iss

#define MyAppName "MiFutbolR"
#define MyAppVersion "0.1"
#define MyAppPublisher "MiFutbolR"
#define MyAppURL "https://github.com/"
#define MyAppExeName "mifutbol-rs.exe"

[Setup]
AppId={{8F3C0B52-7D8E-4C7A-9B2D-6C1E5A4F9D31}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=dist
OutputBaseFilename=MiFutbolR-Setup-{#MyAppVersion}
SetupIconFile=assets\icono.ico
UninstallDisplayIcon={app}\icono.ico
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
VersionInfoVersion=0.1.0.0
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription=Instalador de {#MyAppName}
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#MyAppVersion}
MinVersion=10.0

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Tasks]
Name: "desktopicon"; Description: "Crear un acceso directo en el escritorio"; GroupDescription: "Accesos directos adicionales:"; Flags: unchecked

[Files]
Source: "target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "assets\icono.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "assets\banner.png"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "docs\TUI.md"; DestDir: "{app}\docs"; Flags: ignoreversion
Source: "docs\MIGRACION_BD.md"; DestDir: "{app}\docs"; Flags: ignoreversion
Source: "docs\PLAN_IMPLEMENTACION.md"; DestDir: "{app}\docs"; Flags: ignoreversion
Source: "docs\ACEPTACION.md"; DestDir: "{app}\docs"; Flags: ignoreversion
Source: "docs\img\*.png"; DestDir: "{app}\docs\img"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\icono.ico"; Comment: "Gestión futbolística en terminal"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\icono.ico"; Comment: "Gestión futbolística en terminal"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Iniciar {#MyAppName}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{app}\assets"
Type: filesandordirs; Name: "{app}\docs"
