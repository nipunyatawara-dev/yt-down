Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

Name "YT Down"
OutFile "${OUTPUT_EXE}"
InstallDir "$LOCALAPPDATA\Programs\YT Down"
InstallDirRegKey HKCU "Software\YT Down" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
VIProductVersion "1.0.0.1"
VIAddVersionKey /LANG=1033 "ProductName" "YT Down"
VIAddVersionKey /LANG=1033 "ProductVersion" "1.0 (1)"
VIAddVersionKey /LANG=1033 "FileVersion" "1.0.0.1"
VIAddVersionKey /LANG=1033 "FileDescription" "YT Down Installer"
VIAddVersionKey /LANG=1033 "LegalCopyright" "Nipun Yatawara"
!define MUI_ICON "../src-tauri/icons/icon.ico"
!define MUI_UNICON "../src-tauri/icons/icon.ico"
!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\YT Down.exe"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "YT Down requires 64-bit Windows."
    Abort
  ${EndIf}
  SetRegView 64
FunctionEnd

Section "YT Down"
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  ClearErrors
  File "/oname=YT Down.exe" "${APP_EXE}"
  ${If} ${Errors}
    MessageBox MB_OK|MB_ICONSTOP "Close YT Down before installing this version."
    Abort
  ${EndIf}
  File /oname=WebView2Loader.dll "${WEBVIEW2_LOADER}"
  File /oname=app-icon.png "../src-tauri/icons/128x128.png"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateShortcut "$SMPROGRAMS\YT Down.lnk" "$INSTDIR\YT Down.exe"
  WriteRegStr HKCU "Software\YT Down" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Classes\AppUserModelId\dev.nipunyatawara.ytdown" "DisplayName" "YT Down"
  WriteRegStr HKCU "Software\Classes\AppUserModelId\dev.nipunyatawara.ytdown" "IconUri" "$INSTDIR\app-icon.png"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YT Down" "DisplayName" "YT Down"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YT Down" "DisplayVersion" "1.0 (1)"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YT Down" "Publisher" "Nipun Yatawara"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YT Down" "DisplayIcon" "$INSTDIR\YT Down.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YT Down" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YT Down" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YT Down" "NoRepair" 1

  ; Windows 11 and many Windows 10 PCs already include WebView2.
  ReadRegStr $0 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  ${If} $0 == ""
    ReadRegStr $0 HKCU "Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  ${EndIf}
  ${If} $0 == ""
    InitPluginsDir
    SetOutPath "$PLUGINSDIR"
    File /oname=MicrosoftEdgeWebview2Setup.exe "${WEBVIEW2_BOOTSTRAPPER}"
    DetailPrint "Installing Microsoft Edge WebView2 Runtime..."
    ExecWait '$\"$PLUGINSDIR\MicrosoftEdgeWebview2Setup.exe$\" /silent /install' $1
    ${If} $1 != 0
      MessageBox MB_OK|MB_ICONEXCLAMATION "WebView2 Runtime setup could not finish. Install it from Microsoft's WebView2 download page before launching YT Down."
      ExecShell "open" "https://developer.microsoft.com/microsoft-edge/webview2/#download-section"
    ${EndIf}
    SetOutPath "$INSTDIR"
  ${EndIf}
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  SetRegView 64
  Delete "$SMPROGRAMS\YT Down.lnk"
  Delete "$INSTDIR\YT Down.exe"
  Delete "$INSTDIR\WebView2Loader.dll"
  Delete "$INSTDIR\app-icon.png"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\YT Down"
  DeleteRegKey HKCU "Software\Classes\AppUserModelId\dev.nipunyatawara.ytdown"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YT Down"
SectionEnd
