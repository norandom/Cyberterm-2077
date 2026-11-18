; Per-user installer: no administrator prompt, stable path for self-update.
Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"
Name "Warpeer"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\Warpeer"
InstallDirRegKey HKCU "Software\Warpeer" "InstallDir"
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
  StrCmp $0 "Warpeer" valid
  Abort
  valid:
FunctionEnd

Section "Warpeer"
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  ; Abort safely if Windows prevents overwriting an app that is still running.
  SetOverwrite on
  File /r "${SOURCE}\*"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Warpeer"
  CreateShortcut "$SMPROGRAMS\Warpeer\Warpeer.lnk" "$INSTDIR\warpeer.exe"
  WriteRegStr HKCU "Software\Warpeer" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Warpeer" "DisplayName" "Warpeer"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Warpeer" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Warpeer" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Warpeer" "DisplayIcon" "$INSTDIR\warpeer.exe"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Warpeer" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Warpeer" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  Delete "$SMPROGRAMS\Warpeer\Warpeer.lnk"
  RMDir "$SMPROGRAMS\Warpeer"
  ; Remove only distribution files. User configuration lives outside this folder.
  Delete "$INSTDIR\warpeer.exe"
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
  DeleteRegKey HKCU "Software\Warpeer"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Warpeer"
SectionEnd
