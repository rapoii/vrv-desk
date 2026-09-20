; -- VrV Desk Inno Setup Script --
; Automatically generates a standard Windows Setup Wizard installer (VrV_Desk_Setup_v0.14.0.exe)

#define MyAppName "VrV Desk"
#define MyAppVersion "0.14.0"
#define MyAppPublisher "VrV Desk Team"
#define MyAppURL "https://github.com/rapoii/vrv-desk"
#define MyAppExeName "vrv_desk.exe"

[Setup]
AppId={{D37E84B1-2F10-4A8E-9872-C798B174F81A}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DisableProgramGroupPage=yes
DefaultGroupName={#MyAppName}
LicenseFile=..\LICENSE
OutputDir=..\dist
OutputBaseFilename=VrV_Desk_Setup_v{#MyAppVersion}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "startupicon"; Description: "Start VrV Desk automatically when Windows starts"; GroupDescription: "Startup options:"; Flags: unchecked

[Files]
Source: "..\target\release\vrv_desk.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\vrv_host.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\vrv_signal.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon
Name: "{userstartup}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: startupicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
