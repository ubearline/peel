; peel NSIS installer — install exe + register right-click shell command
!define APPNAME "peel"
!define COMPANY "peel"
!define VERSION "0.1.0"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\peel"

Name "${APPNAME} ${VERSION}"
OutFile "peel-setup.exe"
InstallDir "$PROGRAMFILES64\peel"
RequestExecutionLevel admin
SetCompressor lzma

Page directory
Page instfiles
UninstPage instfiles

Section "Install"
  SetOutPath $INSTDIR
  File "target\x86_64-pc-windows-gnu\release\peel.exe"

  ; Right-click on any file -> peel
  WriteRegStr HKCR "*\shell\peel" "MUIVerb" "Extract with peel"
  WriteRegStr HKCR "*\shell\peel" "Icon" "$INSTDIR\peel.exe"
  WriteRegStr HKCR "*\shell\peel\command" "" '"$INSTDIR\peel.exe" "%1"'

  ; Uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayName" "peel"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "${UNINST_KEY}" "Publisher" "${COMPANY}"
SectionEnd

Section "Uninstall"
  DeleteRegKey HKCR "*\shell\peel"
  DeleteRegKey HKLM "${UNINST_KEY}"
  Delete "$INSTDIR\peel.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
SectionEnd
