; NSIS hooks for the Windows installer (wired through bundle.windows.nsis.installerHooks).

; The updater launches this installer and only then exits the app, so the installer's
; "is Faxina running?" check used to race the app's own shutdown: killing a process that
; is already exiting fails, and the update aborted with "Failed to kill Faxina".
; Wait for the current user's Faxina to go away first, and force it only if it hangs.
!macro NSIS_HOOK_PREINSTALL
  StrCpy $R9 0
  faxina_wait_for_exit:
    nsExec::ExecToStack '"$SYSDIR\cmd.exe" /C ""$SYSDIR\tasklist.exe" /NH /FI "IMAGENAME eq ${MAINBINARYNAME}.exe" /FI "USERNAME eq %USERDOMAIN%\%USERNAME%" | "$SYSDIR\find.exe" /I "${MAINBINARYNAME}.exe""'
    Pop $R8
    Pop $R7
    ${If} $R8 = 0
    ${AndIf} $R9 < 40
      IntOp $R9 $R9 + 1
      Sleep 250
      Goto faxina_wait_for_exit
    ${EndIf}
  ${If} $R8 = 0
    nsExec::ExecToStack '"$SYSDIR\cmd.exe" /C ""$SYSDIR\taskkill.exe" /F /T /IM "${MAINBINARYNAME}.exe" /FI "USERNAME eq %USERDOMAIN%\%USERNAME%""'
    Pop $R8
    Pop $R7
    Sleep 1000
  ${EndIf}
!macroend
