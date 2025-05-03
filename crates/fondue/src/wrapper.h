#pragma once
#include "rust/cxx.h"

rust::Vec<uint8_t> compress_woff2(rust::Slice<const uint8_t> data);
