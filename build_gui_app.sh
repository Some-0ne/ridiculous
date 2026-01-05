#!/bin/bash
# Build Ridiculous GUI as macOS Application

set -e

echo "🔨 Building Ridiculous GUI for macOS..."

# Build release binary with GUI
cargo build --release --features gui

# Create app bundle structure
APP_NAME="Ridiculous.app"
rm -rf "$APP_NAME"
mkdir -p "$APP_NAME/Contents/MacOS"
mkdir -p "$APP_NAME/Contents/Resources"

# Copy binary
cp target/release/ridiculous "$APP_NAME/Contents/MacOS/"

# Create Info.plist
cat > "$APP_NAME/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>ridiculous</string>
    <key>CFBundleIdentifier</key>
    <string>com.ridiculous.gui</string>
    <key>CFBundleName</key>
    <string>Ridiculous</string>
    <key>CFBundleDisplayName</key>
    <string>Ridiculous</string>
    <key>CFBundleVersion</key>
    <string>0.3.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.3.0</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.14</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
</dict>
</plist>
EOF

# Create launcher script that runs with --gui flag
cat > "$APP_NAME/Contents/MacOS/ridiculous-launcher" << 'EOF'
#!/bin/bash
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$DIR/ridiculous-bin" --gui
EOF

# Replace binary with launcher
mv "$APP_NAME/Contents/MacOS/ridiculous" "$APP_NAME/Contents/MacOS/ridiculous-bin"
mv "$APP_NAME/Contents/MacOS/ridiculous-launcher" "$APP_NAME/Contents/MacOS/ridiculous"
chmod +x "$APP_NAME/Contents/MacOS/ridiculous"
chmod +x "$APP_NAME/Contents/MacOS/ridiculous-bin"

echo "✅ Created $APP_NAME"
echo ""
echo "📦 You can now:"
echo "   1. Open it: open $APP_NAME"
echo "   2. Drag it to /Applications"
echo "   3. Or move it: mv $APP_NAME /Applications/"
echo ""
