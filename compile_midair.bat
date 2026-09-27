@echo off
setlocal
echo ============================================================
echo   SCRIPT MIDAIR  -  Compilation (Rust)
echo ============================================================
echo.

cd /d "%~dp0"

where cargo >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo ERREUR: Rust n'est pas installe ou n'est pas dans le PATH.
    echo Installe-le depuis https://rustup.rs puis relance ce script.
    pause
    exit /b 1
)

echo [1/4] Version de Rust...
cargo --version
rustc --version
echo.

echo [2/4] Tests de non-regression...
cargo test --bin ScriptMidair --release
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ERREUR: les tests ont echoue, compilation annulee.
    pause
    exit /b 1
)
echo OK
echo.

echo [3/4] Compilation optimisee...
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ERREUR de compilation
    pause
    exit /b 1
)
echo OK
echo.

echo [4/4] Copie de l'executable...
if not exist dist mkdir dist
copy /y "target\release\ScriptMidair.exe" "dist\ScriptMidair.exe" >nul
echo OK
echo.

echo ============================================================
echo   COMPILATION REUSSIE
echo   Fichier : %CD%\dist\ScriptMidair.exe
echo ============================================================
echo.
pause
