#include "wrapper.h"
#include "woff2/encode.h"

rust::Vec<uint8_t> compress_woff2(rust::Slice<const uint8_t> data) {
  const uint8_t* input_data = data.data();
  size_t input_size = data.size();

  size_t output_size = woff2::MaxWOFF2CompressedSize(input_data, input_size);
  std::vector<uint8_t> output_vec(output_size);

  if (!woff2::ConvertTTFToWOFF2(input_data, input_size, output_vec.data(), &output_size)) {
    return rust::Vec<uint8_t>();
  }

  output_vec.resize(output_size);

  rust::Vec<uint8_t> result;
  result.reserve(output_vec.size());
  for (const auto& byte : output_vec) {
    result.push_back(byte);
  }

  return result;
}