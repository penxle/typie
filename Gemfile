source "https://rubygems.org"

gem "abbrev"
gem "csv"
gem "fastlane"
gem "fastlane-plugin-sentry"
gem "ostruct"

plugins_path = File.join(File.dirname(__FILE__), 'fastlane', 'Pluginfile')
eval_gemfile(plugins_path) if File.exist?(plugins_path)
