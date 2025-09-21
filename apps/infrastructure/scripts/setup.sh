#!/bin/bash

brew install cirruslabs/cli/tart
tart create talos --linux --disk-size=100 --disk-format=asif

mkdir -p "$HOME/Library/LaunchAgents"
cat > "$HOME/Library/LaunchAgents/io.typie.tart.talos.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>io.typie.tart.talos</string>
    <key>ProgramArguments</key>
    <array>
        <string>/opt/homebrew/bin/tart</string>
        <string>run</string>
        <string>talos</string>
        <string>--net-bridged=en0</string>
        <string>--dir=/Volumes/Data/PV</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
        <key>Crashed</key>
        <true/>
    </dict>
    <key>StandardOutPath</key>
    <string>/Users/user/Library/Logs/tart-talos.log</string>
    <key>StandardErrorPath</key>
    <string>/Users/user/Library/Logs/tart-talos-error.log</string>
    <key>ProcessType</key>
    <string>Interactive</string>
    <key>LimitLoadToSessionType</key>
    <array>
        <string>GUI</string>
        <string>Aqua</string>
    </array>
</dict>
</plist>
EOF

chmod 644 "$HOME/Library/LaunchAgents/io.typie.tart.talos.plist"
launchctl load "$HOME/Library/LaunchAgents/io.typie.tart.talos.plist"

mkdir /Volumes/Data/PV

echo "tart run talos --disk=./Downloads/talos.iso --net-bridged=en0 --dir=/Volumes/Data/PV"
