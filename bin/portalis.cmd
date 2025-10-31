@echo off
REM Portalis wrapper script for Windows

set bindir=%~dp0
set exe=%bindir%portalis.exe

if not exist "%exe%" (
  echo Error: Portalis binary not found at %exe%
  echo Try reinstalling: npm install -g portalis
  exit /b 1
)

"%exe%" %*
