#!/usr/bin/env bash
set -euo pipefail
export PATH="/opt/homebrew/bin:/usr/local/bin:$PATH"
cd "$(dirname "$0")/.."

case "${1:-}" in
  Debug | dev) config=dev ;;
  Release | prod) config=prod ;;
  Profile) config=dev ;;
  *)
    echo "unknown configuration: ${1:-}" >&2
    exit 1
    ;;
esac

keys='["API_URL","AUTH_URL","OIDC_CLIENT_ID","OIDC_CLIENT_SECRET","GOOGLE_IOS_CLIENT_ID","GOOGLE_DOT_REVERSED_IOS_CLIENT_ID","GOOGLE_SERVER_CLIENT_ID","KAKAO_NATIVE_APP_KEY","NAVER_CLIENT_ID","NAVER_CLIENT_SECRET"]'
config_tmp=$(mktemp)
trap 'rm -f "$config_tmp"' EXIT
downloaded=$(env -i PATH="$PATH" HOME="$HOME" ${DOPPLER_TOKEN:+DOPPLER_TOKEN="$DOPPLER_TOKEN"} doppler secrets download -p mobile -c "$config" --no-file --format json)
printf '%s' "$downloaded" | jq -r --argjson keys "$keys" '
    . as $all
    | $keys[]
    | . as $key
    | $all[$key]
    | if type != "string" or test("^\\s*$") then error("missing \($key)") else . end
    | if test("[$\n]") then error("unsupported character in \($key)") else . end
    | if test("^\\s|\\s$|\\r") then error("unsupported whitespace in \($key)") else . end
    | if ($key | IN("API_URL", "AUTH_URL")) and (test("^https?://[^/]+") | not) then error("invalid URL in \($key)") else . end
    | "\($key) = \(gsub("//"; "/$()/"))"
  ' > "$config_tmp"
[ "$(wc -l < "$config_tmp" | tr -d ' ')" -eq "$(jq length <<< "$keys")" ]
chmod 600 "$config_tmp"
mv "$config_tmp" Configuration/Config.local.xcconfig
echo "wrote Configuration/Config.local.xcconfig ($(wc -l < Configuration/Config.local.xcconfig | tr -d ' ') keys)"
