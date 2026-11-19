; Per-user installer: no administrator prompt, stable path for self-update.
Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"
Name "Cyberterm 2077"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\Cyberterm 2077"
InstallDirRegKey HKCU "Software\Cyberterm 2077" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${SOURCE}\LICENSE-AGPL"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Function .onVerifyInstDir
  ${GetFileName} "$INSTDIR" $0
  StrCmp $0 "Cyberterm 2077" valid
  Abort
  valid:
FunctionEnd

Section "Cyberterm 2077"
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  ; Abort safely if Windows prevents overwriting an app that is still running.
  SetOverwrite on
  File /r "${SOURCE}\*"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Cyberterm 2077"
  CreateShortcut "$SMPROGRAMS\Cyberterm 2077\Cyberterm 2077.lnk" "$INSTDIR\cyberterm2077.exe"
  WriteRegStr HKCU "Software\Cyberterm 2077" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cyberterm 2077" "DisplayName" "Cyberterm 2077"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cyberterm 2077" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cyberterm 2077" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cyberterm 2077" "DisplayIcon" "$INSTDIR\cyberterm2077.exe"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cyberterm 2077" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cyberterm 2077" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  Delete "$SMPROGRAMS\Cyberterm 2077\Cyberterm 2077.lnk"
  RMDir "$SMPROGRAMS\Cyberterm 2077"
  ; Remove only distribution files. User configuration lives outside this folder.
  Delete "$INSTDIR\cyberterm2077.exe"
  Delete "$INSTDIR\warpeer-update-helper.exe"
  Delete "$INSTDIR\conpty.dll"
  Delete "$INSTDIR\dxcompiler.dll"
  Delete "$INSTDIR\dxil.dll"
  Delete "$INSTDIR\msvcp140.dll"
  Delete "$INSTDIR\vcruntime140.dll"
  Delete "$INSTDIR\vcruntime140_1.dll"
  Delete "$INSTDIR\LICENSE-AGPL"
  Delete "$INSTDIR\LICENSE-MIT"
  Delete "$INSTDIR\NOTICE"
  Delete "$INSTDIR\.warpeer-update-error"
  RMDir /r "$INSTDIR\resources"
  RMDir /r "$INSTDIR\x64"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Cyberterm 2077"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cyberterm 2077"
SectionEnd
