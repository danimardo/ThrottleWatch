; T157 (FR-089): PawnIO is a shared kernel driver other applications may also depend on, so
; ThrottleWatch's own uninstaller never removes or touches it — this hook only informs the
; person how to uninstall it separately, in whichever of the two installer languages they picked
; (`bundle.windows.nsis.languages` in tauri.conf.json: English, Spanish). No automatic action.
;
; $LANGUAGE is a plain NSIS integer at this point (LCID), compared directly against the two
; installer languages' LCIDs so this file needs no `!include` of its own and does not depend on
; where in Tauri's base template the hook point lands.
!macro NSIS_HOOK_PREUNINSTALL
  IntCmp $LANGUAGE 1034 pawnio_notice_es pawnio_notice_en pawnio_notice_en
  pawnio_notice_es:
    MessageBox MB_OK|MB_ICONINFORMATION "ThrottleWatch no elimina PawnIO: es un controlador compartido que pueden usar otras aplicaciones.$\r$\n$\r$\nPara desinstalarlo aparte, usa Configuración de Windows > Aplicaciones > PawnIO, o ejecuta su propio desinstalador desde Archivos de programa\PawnIO." /SD IDOK
    Goto pawnio_notice_done
  pawnio_notice_en:
    MessageBox MB_OK|MB_ICONINFORMATION "ThrottleWatch does not remove PawnIO: it is a shared driver other applications may also depend on.$\r$\n$\r$\nTo uninstall it separately, use Windows Settings > Apps > PawnIO, or run its own uninstaller from Program Files\PawnIO." /SD IDOK
  pawnio_notice_done:
!macroend
