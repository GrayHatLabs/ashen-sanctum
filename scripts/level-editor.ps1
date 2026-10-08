# Open the level editor in your default browser (Chrome or Edge recommended: they can save
# straight into the levels/ folder). Refreshes the editor's catalog from the game when a WSL build
# exists, and the sprite thumbnails.
$bin = '/root/.cache/ashensanctum-target/release/ashensanctum'
wsl -d Ubuntu -u root -- bash -c "cd /mnt/d/projects/AshenSanctum && test -x $bin && $bin --export-catalog tools/level-editor/catalog.js" | Out-Null
python D:\projects\AshenSanctum\tools\level-editor\make_thumbs.py | Out-Null
Start-Process D:\projects\AshenSanctum\tools\level-editor\index.html
