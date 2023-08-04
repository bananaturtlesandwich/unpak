#pragma once

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <emmintrin.h>

#define assert(x) (void)0

#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define MIN(a, b) ((a) < (b) ? (a) : (b))

inline uint16_t
byteswap_ushort (uint16_t i)
{
  #ifdef __linux__
  uint16_t j;
  j = (i << 8);
  j += (i >> 8);
  return j;
  #else
  return _byteswap_ushort ((unsigned short)i);
  #endif
}

inline uint32_t
byteswap_ulong (uint32_t i)
{
  #ifdef __linux__
  uint32_t j;
  j = (i << 24);
  j += (i << 8) & 0x00FF0000;
  j += (i >> 8) & 0x0000FF00;
  j += (i >> 24);
  return j;
  #else
  return _byteswap_ulong ((unsigned long)i);
  #endif
}

inline uint64_t
byteswap_uint64 (uint64_t i)
{
  #ifdef __linux__
  uint64_t j;
  j = (i << 56);
  j += (i << 40) & 0x00FF000000000000;
  j += (i << 24) & 0x0000FF0000000000;
  j += (i << 8) & 0x000000FF00000000;
  j += (i >> 8) & 0x00000000FF000000;
  j += (i >> 24) & 0x0000000000FF0000;
  j += (i >> 40) & 0x000000000000FF00;
  j += (i >> 56);
  return j;
  #else
  return _byteswap_uint64 ((unsigned long long)i);
  #endif
}

#ifdef __linux__
// GCC __forceinline macro
#define __forceinline inline __attribute__ ((always_inline))
#endif

__forceinline uint8_t
BitScanReverse (uint64_t *const Index, const uint64_t Mask)
{
  #ifdef __linux__
  *Index = 31 - __builtin_clz (Mask);
  return Mask ? 1 : 0;
  #else
  return _BitScanReverse64 ((unsigned long *)Index, Mask);
  #endif
}

__forceinline uint8_t
BitScanForward (uint64_t *const Index, const uint64_t Mask)
{
  #ifdef __linux__
  *Index = __builtin_ctz (Mask);
  return Mask ? 1 : 0;
  #else
  return _BitScanForward64 ((unsigned long *)Index, Mask);
  #endif
}

__forceinline uint32_t
rotl (uint32_t value, int32_t shift)
{
  #ifdef __linux__
  return (((value) << ((int32_t)(shift)))
          | ((value) >> (32 - (int32_t)(shift))));
  #else
  return _rotl ((unsigned int)value, (int)shift);
  #endif
}
