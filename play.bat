@echo off
rem Ashen Sanctum launcher: double-click to play (opens the title screen).
rem Extra options can follow on a command line, e.g.  play.bat --act4 --reaper
title Ashen Sanctum
wsl -d Ubuntu -e bash -lc "cd /mnt/d/projects/AshenSanctum && bash scripts/dev-build.sh run %*"
if errorlevel 1 pause
