#pragma once

#include "stdafx.h"

typedef uint16_t LznaBitModel;

// State for a 4-bit value RANS model
struct LznaNibbleModel
{
  uint16_t prob[17];
};

// State for a 3-bit value RANS model
struct Lzna3bitModel
{
  uint16_t prob[9];
};

// State for the literal model
struct LznaLiteralModel
{
  LznaNibbleModel upper[16];
  LznaNibbleModel lower[16];
  LznaNibbleModel nomatch[16];
};

// State for a model representing a far distance
struct LznaFarDistModel
{
  LznaNibbleModel first_lo;
  LznaNibbleModel first_hi;
  LznaBitModel second[31];
  LznaBitModel third[2][31];
};

// State for a model representing a near distance
struct LznaNearDistModel
{
  LznaNibbleModel first;
  LznaBitModel second[16];
  LznaBitModel third[2][16];
};

// State for model representing the low bits of a distance
struct LznaLowBitsDistanceModel
{
  LznaNibbleModel d[2];
  LznaBitModel v;
};

// State for model used for the short lengths for recent matches
struct LznaShortLengthRecentModel
{
  Lzna3bitModel a[4];
};

// State for model for long lengths
struct LznaLongLengthModel
{
  LznaNibbleModel first[4];
  LznaNibbleModel second;
  LznaNibbleModel third;
};

// Complete LZNA state
struct LznaState
{
  uint32_t match_history[8];
  LznaLiteralModel literal[4];
  LznaBitModel is_literal[12 * 8];
  LznaNibbleModel type[12 * 8];
  LznaShortLengthRecentModel short_length_recent[4];
  LznaLongLengthModel long_length_recent;
  LznaLowBitsDistanceModel low_bits_of_distance[2];
  LznaBitModel short_length[12][4];
  LznaNearDistModel near_dist[2];
  Lzna3bitModel medium_length;
  LznaLongLengthModel long_length;
  LznaFarDistModel far_distance;
};

struct LznaBitReader
{
  uint64_t bits_a, bits_b;
  const uint32_t *src, *src_start;
};

static LznaNibbleModel lzna_initializer_4bit = {
  0x0,    0x800,  0x1000, 0x1800, 0x2000, 0x2800, 0x3000, 0x3800, 0x4000,
  0x4800, 0x5000, 0x5800, 0x6000, 0x6800, 0x7000, 0x7800, 0x8000,
};

static Lzna3bitModel lzna_initializer_3bit
    = { 0x0, 0x1000, 0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 0x7000, 0x8000 };

static const uint8_t next_state_lit[12]
    = { 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 4, 5 };

void LZNA_InitLookup (LznaState *lut);

int LZNA_DecodeQuantum (uint8_t *dst, uint8_t *dst_end, uint8_t *dst_start,
                        const uint8_t *src_in, const uint8_t *src_end,
                        LznaState *lut);
