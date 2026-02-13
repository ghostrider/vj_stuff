/*{
  "ISFVSN": "2",
  "VSN": "1.0",
  "DESCRIPTION": "GLIC Haar Wavelet Glitch — Block-based Haar wavelet decomposition with per-subband manipulation. Amplify, zero, or quantize frequency bands for wavelet glitch effects.",
  "CREDIT": "GLIC port by Claude",
  "CATEGORIES": ["GLIC"],
  "INPUTS": [
    {
      "NAME": "inputImage",
      "TYPE": "image"
    },
    {
      "NAME": "decompLevel",
      "TYPE": "long",
      "VALUES": [1,2,3,4],
      "LABELS": ["Level 1 (2x2)","Level 2 (4x4)","Level 3 (8x8)","Level 4 (16x16)"],
      "DEFAULT": 2
    },
    {
      "NAME": "gainLL",
      "TYPE": "float",
      "DEFAULT": 1.0,
      "MIN": 0.0,
      "MAX": 4.0
    },
    {
      "NAME": "gainLH",
      "TYPE": "float",
      "DEFAULT": 1.0,
      "MIN": 0.0,
      "MAX": 8.0
    },
    {
      "NAME": "gainHL",
      "TYPE": "float",
      "DEFAULT": 1.0,
      "MIN": 0.0,
      "MAX": 8.0
    },
    {
      "NAME": "gainHH",
      "TYPE": "float",
      "DEFAULT": 1.0,
      "MIN": 0.0,
      "MAX": 8.0
    },
    {
      "NAME": "quantCoeffs",
      "TYPE": "float",
      "DEFAULT": 0.0,
      "MIN": 0.0,
      "MAX": 64.0
    },
    {
      "NAME": "vizMode",
      "TYPE": "long",
      "VALUES": [0,1,2],
      "LABELS": ["Reconstruct","Show Subbands","Coefficients"],
      "DEFAULT": 0
    },
    {
      "NAME": "intensity",
      "TYPE": "float",
      "DEFAULT": 1.0,
      "MIN": 0.0,
      "MAX": 1.0
    }
  ]
}*/

// ============================================================
// Single-pass block-based Haar wavelet transform
//
// Instead of multi-pass ping-pong (unreliable in Wire), we compute
// the full Haar decomposition within a local block for each pixel.
//
// For level N, the block is 2^N x 2^N pixels. We read all pixels
// in the block, compute the wavelet coefficients in-shader, apply
// manipulations, and reconstruct.
//
// This is more texture reads per pixel but avoids multi-pass entirely.
// Level 4 (16x16 = 256 reads) is the practical limit.
// ============================================================

// Sample at pixel coordinates
vec3 samplePx(vec2 p) {
  return IMG_NORM_PIXEL(inputImage, clamp(p / RENDERSIZE, 0.0, 1.0)).rgb;
}

// ============================================================
// 1D Haar forward transform (in-place on array via output)
// For length N: averages go to first N/2, details to last N/2
// ============================================================

// Level 1: 2-element Haar
void haar1d_2(inout vec3 a, inout vec3 b) {
  vec3 avg = (a + b) * 0.5;
  vec3 det = (a - b) * 0.5;
  a = avg;
  b = det;
}

// ============================================================
// 2D Haar transform for 2x2 block (Level 1)
// Returns: LL, LH, HL, HH subbands
// ============================================================

void haar2d_level1(vec3 s00, vec3 s10, vec3 s01, vec3 s11,
                   out vec3 ll, out vec3 lh, out vec3 hl, out vec3 hh) {
  // Horizontal transform on each row
  vec3 r0a = (s00 + s10) * 0.5;
  vec3 r0d = (s00 - s10) * 0.5;
  vec3 r1a = (s01 + s11) * 0.5;
  vec3 r1d = (s01 - s11) * 0.5;

  // Vertical transform on columns
  ll = (r0a + r1a) * 0.5;
  hl = (r0a - r1a) * 0.5;
  lh = (r0d + r1d) * 0.5;
  hh = (r0d - r1d) * 0.5;
}

// Inverse 2D Haar for 2x2
void ihaar2d_level1(vec3 ll, vec3 lh, vec3 hl, vec3 hh,
                    out vec3 s00, out vec3 s10, out vec3 s01, out vec3 s11) {
  // Inverse vertical
  vec3 r0a = ll + hl;
  vec3 r1a = ll - hl;
  vec3 r0d = lh + hh;
  vec3 r1d = lh - hh;

  // Inverse horizontal
  s00 = r0a + r0d;
  s10 = r0a - r0d;
  s01 = r1a + r1d;
  s11 = r1a - r1d;
}

// ============================================================
// Quantize wavelet coefficients
// ============================================================

vec3 quantize(vec3 v, float step) {
  if (step < 0.5) return v;
  return floor(v * 255.0 / step + 0.5) * step / 255.0;
}

// ============================================================
// Main: block-based Haar with arbitrary depth
//
// For each decomposition level, we read a 2^L x 2^L block,
// compute 2D Haar, and manipulate subbands.
//
// We compute the full transform by iterating:
// 1. Read all pixels in block
// 2. Apply horizontal Haar to each row
// 3. Apply vertical Haar to each column
// 4. Repeat on the LL (top-left) quadrant for deeper levels
// ============================================================

void main() {
  vec2 uv = isf_FragNormCoord;
  vec4 original = IMG_NORM_PIXEL(inputImage, uv);
  vec3 rgb = original.rgb;

  vec2 pixelPos = uv * RENDERSIZE;
  int blockSize = int(pow(2.0, float(decompLevel)));
  float fBlockSize = float(blockSize);

  // Find which block this pixel belongs to
  vec2 blockOrigin = floor(pixelPos / fBlockSize) * fBlockSize;

  // Position within the block (0 to blockSize-1)
  vec2 localPos = pixelPos - blockOrigin;
  int lx = int(localPos.x);
  int ly = int(localPos.y);

  // ---- Read all pixels in the block into a working array ----
  // We use a flat approach: compute Haar coefficients directly.
  //
  // For levels 1-2, we do it analytically.
  // For levels 3-4, we use iterative 1D transforms.

  vec3 result = rgb;

  if (decompLevel == 1) {
    // 2x2 block: 4 samples
    vec3 s00 = samplePx(blockOrigin + vec2(0.0, 0.0));
    vec3 s10 = samplePx(blockOrigin + vec2(1.0, 0.0));
    vec3 s01 = samplePx(blockOrigin + vec2(0.0, 1.0));
    vec3 s11 = samplePx(blockOrigin + vec2(1.0, 1.0));

    vec3 ll, lh, hl, hh;
    haar2d_level1(s00, s10, s01, s11, ll, lh, hl, hh);

    // Apply gains
    ll *= gainLL;
    lh *= gainLH;
    hl *= gainHL;
    hh *= gainHH;

    // Quantize coefficients
    if (quantCoeffs > 0.5) {
      lh = quantize(lh, quantCoeffs);
      hl = quantize(hl, quantCoeffs);
      hh = quantize(hh, quantCoeffs);
    }

    if (vizMode == 1) {
      // Show subbands in quadrants
      if (lx == 0 && ly == 0) result = ll + 0.5;
      else if (lx == 1 && ly == 0) result = lh + 0.5;
      else if (lx == 0 && ly == 1) result = hl + 0.5;
      else result = hh + 0.5;
    } else if (vizMode == 2) {
      // Show coefficient magnitude
      vec3 mag = abs(lh) + abs(hl) + abs(hh);
      result = mag * 4.0;
    } else {
      // Reconstruct
      vec3 r00, r10, r01, r11;
      ihaar2d_level1(ll, lh, hl, hh, r00, r10, r01, r11);
      if (lx == 0 && ly == 0) result = r00;
      else if (lx == 1 && ly == 0) result = r10;
      else if (lx == 0 && ly == 1) result = r01;
      else result = r11;
    }

  } else if (decompLevel == 2) {
    // 4x4 block: 16 samples
    // Read all 16 pixels
    vec3 s[16]; // but GLSL ES doesn't have variable arrays well, use explicit
    vec3 s00 = samplePx(blockOrigin + vec2(0,0));
    vec3 s10 = samplePx(blockOrigin + vec2(1,0));
    vec3 s20 = samplePx(blockOrigin + vec2(2,0));
    vec3 s30 = samplePx(blockOrigin + vec2(3,0));
    vec3 s01 = samplePx(blockOrigin + vec2(0,1));
    vec3 s11 = samplePx(blockOrigin + vec2(1,1));
    vec3 s21 = samplePx(blockOrigin + vec2(2,1));
    vec3 s31 = samplePx(blockOrigin + vec2(3,1));
    vec3 s02 = samplePx(blockOrigin + vec2(0,2));
    vec3 s12 = samplePx(blockOrigin + vec2(1,2));
    vec3 s22 = samplePx(blockOrigin + vec2(2,2));
    vec3 s32 = samplePx(blockOrigin + vec2(3,2));
    vec3 s03 = samplePx(blockOrigin + vec2(0,3));
    vec3 s13 = samplePx(blockOrigin + vec2(1,3));
    vec3 s23 = samplePx(blockOrigin + vec2(2,3));
    vec3 s33 = samplePx(blockOrigin + vec2(3,3));

    // Level 1: Horizontal Haar on each row (pairs)
    // Row 0
    vec3 r0_a0 = (s00+s10)*0.5; vec3 r0_d0 = (s00-s10)*0.5;
    vec3 r0_a1 = (s20+s30)*0.5; vec3 r0_d1 = (s20-s30)*0.5;
    // Row 1
    vec3 r1_a0 = (s01+s11)*0.5; vec3 r1_d0 = (s01-s11)*0.5;
    vec3 r1_a1 = (s21+s31)*0.5; vec3 r1_d1 = (s21-s31)*0.5;
    // Row 2
    vec3 r2_a0 = (s02+s12)*0.5; vec3 r2_d0 = (s02-s12)*0.5;
    vec3 r2_a1 = (s22+s32)*0.5; vec3 r2_d1 = (s22-s32)*0.5;
    // Row 3
    vec3 r3_a0 = (s03+s13)*0.5; vec3 r3_d0 = (s03-s13)*0.5;
    vec3 r3_a1 = (s23+s33)*0.5; vec3 r3_d1 = (s23-s33)*0.5;

    // Level 1: Vertical Haar on columns (pairs of rows)
    // Average columns (left half: a0s)
    vec3 c_aa00 = (r0_a0+r1_a0)*0.5; vec3 c_da00 = (r0_a0-r1_a0)*0.5;
    vec3 c_aa10 = (r0_a1+r1_a1)*0.5; vec3 c_da10 = (r0_a1-r1_a1)*0.5;
    vec3 c_aa01 = (r2_a0+r3_a0)*0.5; vec3 c_da01 = (r2_a0-r3_a0)*0.5;
    vec3 c_aa11 = (r2_a1+r3_a1)*0.5; vec3 c_da11 = (r2_a1-r3_a1)*0.5;
    // Detail columns (right half: d0s)
    vec3 c_ad00 = (r0_d0+r1_d0)*0.5; vec3 c_dd00 = (r0_d0-r1_d0)*0.5;
    vec3 c_ad10 = (r0_d1+r1_d1)*0.5; vec3 c_dd10 = (r0_d1-r1_d1)*0.5;
    vec3 c_ad01 = (r2_d0+r3_d0)*0.5; vec3 c_dd01 = (r2_d0-r3_d0)*0.5;
    vec3 c_ad11 = (r2_d1+r3_d1)*0.5; vec3 c_dd11 = (r2_d1-r3_d1)*0.5;

    // After level 1, we have a 4x4 grid of coefficients:
    // Top-left 2x2 = LL subband (averages)
    // Top-right 2x2 = LH subband (horizontal detail)
    // Bottom-left 2x2 = HL subband (vertical detail)
    // Bottom-right 2x2 = HH subband (diagonal detail)

    // Level 2: Apply Haar again on the LL 2x2 block
    vec3 ll2, lh2, hl2, hh2;
    haar2d_level1(c_aa00, c_aa10, c_aa01, c_aa11, ll2, lh2, hl2, hh2);

    // Apply gains to Level-2 subbands (the deepest decomposition)
    ll2 *= gainLL;
    lh2 *= gainLH;
    hl2 *= gainHL;
    hh2 *= gainHH;

    // Apply gains to Level-1 detail subbands (scale by detail gains)
    float detailGain = (gainLH + gainHL + gainHH) / 3.0;
    // LH band (top-right 2x2)
    c_ad00 *= gainLH; c_ad10 *= gainLH;
    c_ad01 *= gainLH; c_ad11 *= gainLH;
    // HL band (bottom-left 2x2)
    c_da00 *= gainHL; c_da10 *= gainHL;
    c_da01 *= gainHL; c_da11 *= gainHL;
    // HH band (bottom-right 2x2)
    c_dd00 *= gainHH; c_dd10 *= gainHH;
    c_dd01 *= gainHH; c_dd11 *= gainHH;

    // Quantize coefficients
    if (quantCoeffs > 0.5) {
      lh2 = quantize(lh2, quantCoeffs);
      hl2 = quantize(hl2, quantCoeffs);
      hh2 = quantize(hh2, quantCoeffs);
      c_ad00 = quantize(c_ad00, quantCoeffs); c_ad10 = quantize(c_ad10, quantCoeffs);
      c_ad01 = quantize(c_ad01, quantCoeffs); c_ad11 = quantize(c_ad11, quantCoeffs);
      c_da00 = quantize(c_da00, quantCoeffs); c_da10 = quantize(c_da10, quantCoeffs);
      c_da01 = quantize(c_da01, quantCoeffs); c_da11 = quantize(c_da11, quantCoeffs);
      c_dd00 = quantize(c_dd00, quantCoeffs); c_dd10 = quantize(c_dd10, quantCoeffs);
      c_dd01 = quantize(c_dd01, quantCoeffs); c_dd11 = quantize(c_dd11, quantCoeffs);
    }

    if (vizMode == 1) {
      // Subband visualization: map pixel position to subband
      // 4x4 grid: top-left 2x2 = LL decomposed, etc.
      vec3 val = vec3(0.0);
      if (lx < 2 && ly < 2) {
        // LL quadrant — show level-2 decomposition
        if (lx == 0 && ly == 0) val = ll2;
        else if (lx == 1 && ly == 0) val = lh2;
        else if (lx == 0 && ly == 1) val = hl2;
        else val = hh2;
      } else if (lx >= 2 && ly < 2) {
        // LH quadrant
        int sx = lx - 2; int sy = ly;
        if (sx == 0 && sy == 0) val = c_ad00;
        else if (sx == 1 && sy == 0) val = c_ad10;
        else if (sx == 0 && sy == 1) val = c_ad01;
        else val = c_ad11;
      } else if (lx < 2 && ly >= 2) {
        // HL quadrant
        int sx = lx; int sy = ly - 2;
        if (sx == 0 && sy == 0) val = c_da00;
        else if (sx == 1 && sy == 0) val = c_da10;
        else if (sx == 0 && sy == 1) val = c_da01;
        else val = c_da11;
      } else {
        // HH quadrant
        int sx = lx - 2; int sy = ly - 2;
        if (sx == 0 && sy == 0) val = c_dd00;
        else if (sx == 1 && sy == 0) val = c_dd10;
        else if (sx == 0 && sy == 1) val = c_dd01;
        else val = c_dd11;
      }
      result = val + 0.5;

    } else if (vizMode == 2) {
      // Coefficient magnitude map
      vec3 mag = abs(lh2) + abs(hl2) + abs(hh2);
      mag += abs(c_ad00) + abs(c_da00) + abs(c_dd00);
      result = mag * 3.0;

    } else {
      // Reconstruct: inverse level-2 Haar on LL block
      ihaar2d_level1(ll2, lh2, hl2, hh2, c_aa00, c_aa10, c_aa01, c_aa11);

      // Inverse level-1: undo vertical
      vec3 ir0_a0 = c_aa00 + c_da00; vec3 ir1_a0 = c_aa00 - c_da00;
      vec3 ir0_a1 = c_aa10 + c_da10; vec3 ir1_a1 = c_aa10 - c_da10;
      vec3 ir2_a0 = c_aa01 + c_da01; vec3 ir3_a0 = c_aa01 - c_da01;
      vec3 ir2_a1 = c_aa11 + c_da11; vec3 ir3_a1 = c_aa11 - c_da11;

      vec3 ir0_d0 = c_ad00 + c_dd00; vec3 ir1_d0 = c_ad00 - c_dd00;
      vec3 ir0_d1 = c_ad10 + c_dd10; vec3 ir1_d1 = c_ad10 - c_dd10;
      vec3 ir2_d0 = c_ad01 + c_dd01; vec3 ir3_d0 = c_ad01 - c_dd01;
      vec3 ir2_d1 = c_ad11 + c_dd11; vec3 ir3_d1 = c_ad11 - c_dd11;

      // Inverse level-1: undo horizontal
      // Reconstruct all 16 pixels
      vec3 p00 = ir0_a0+ir0_d0; vec3 p10 = ir0_a0-ir0_d0;
      vec3 p20 = ir0_a1+ir0_d1; vec3 p30 = ir0_a1-ir0_d1;
      vec3 p01 = ir1_a0+ir1_d0; vec3 p11 = ir1_a0-ir1_d0;
      vec3 p21 = ir1_a1+ir1_d1; vec3 p31 = ir1_a1-ir1_d1;
      vec3 p02 = ir2_a0+ir2_d0; vec3 p12 = ir2_a0-ir2_d0;
      vec3 p22 = ir2_a1+ir2_d1; vec3 p32 = ir2_a1-ir2_d1;
      vec3 p03 = ir3_a0+ir3_d0; vec3 p13 = ir3_a0-ir3_d0;
      vec3 p23 = ir3_a1+ir3_d1; vec3 p33 = ir3_a1-ir3_d1;

      // Select the pixel
      if      (lx==0 && ly==0) result = p00;
      else if (lx==1 && ly==0) result = p10;
      else if (lx==2 && ly==0) result = p20;
      else if (lx==3 && ly==0) result = p30;
      else if (lx==0 && ly==1) result = p01;
      else if (lx==1 && ly==1) result = p11;
      else if (lx==2 && ly==1) result = p21;
      else if (lx==3 && ly==1) result = p31;
      else if (lx==0 && ly==2) result = p02;
      else if (lx==1 && ly==2) result = p12;
      else if (lx==2 && ly==2) result = p22;
      else if (lx==3 && ly==2) result = p32;
      else if (lx==0 && ly==3) result = p03;
      else if (lx==1 && ly==3) result = p13;
      else if (lx==2 && ly==3) result = p23;
      else                     result = p33;
    }

  } else {
    // Levels 3-4: Use averaged 2x2 sub-blocks to reduce to a 4x4 or 2x2 problem
    // This is an approximation: we average each 2^(L-2) x 2^(L-2) sub-block
    // then apply the level-2 Haar decomposition on those averages

    int subBlockSize = blockSize / 4;
    float fSubBlock = float(subBlockSize);

    // Compute 4x4 grid of sub-block averages
    vec3 grid[4];  // We'll compute row by row
    // Which sub-block does this pixel belong to?
    int gridX = int(localPos.x / fSubBlock);
    int gridY = int(localPos.y / fSubBlock);
    gridX = min(gridX, 3);
    gridY = min(gridY, 3);

    // Read 4x4 grid of averages
    vec3 g00=vec3(0.0), g10=vec3(0.0), g20=vec3(0.0), g30=vec3(0.0);
    vec3 g01=vec3(0.0), g11=vec3(0.0), g21=vec3(0.0), g31=vec3(0.0);
    vec3 g02=vec3(0.0), g12=vec3(0.0), g22=vec3(0.0), g32=vec3(0.0);
    vec3 g03=vec3(0.0), g13=vec3(0.0), g23=vec3(0.0), g33=vec3(0.0);

    // Average each sub-block with sparse sampling (4 samples per sub-block)
    float halfSub = fSubBlock * 0.5;
    float qSub = fSubBlock * 0.25;

    for (int gy = 0; gy < 4; gy++) {
      for (int gx = 0; gx < 4; gx++) {
        vec2 subOrigin = blockOrigin + vec2(float(gx), float(gy)) * fSubBlock;
        vec3 avg = vec3(0.0);
        avg += samplePx(subOrigin + vec2(qSub, qSub));
        avg += samplePx(subOrigin + vec2(qSub*3.0, qSub));
        avg += samplePx(subOrigin + vec2(qSub, qSub*3.0));
        avg += samplePx(subOrigin + vec2(qSub*3.0, qSub*3.0));
        avg *= 0.25;

        if      (gx==0 && gy==0) g00 = avg;
        else if (gx==1 && gy==0) g10 = avg;
        else if (gx==2 && gy==0) g20 = avg;
        else if (gx==3 && gy==0) g30 = avg;
        else if (gx==0 && gy==1) g01 = avg;
        else if (gx==1 && gy==1) g11 = avg;
        else if (gx==2 && gy==1) g21 = avg;
        else if (gx==3 && gy==1) g31 = avg;
        else if (gx==0 && gy==2) g02 = avg;
        else if (gx==1 && gy==2) g12 = avg;
        else if (gx==2 && gy==2) g22 = avg;
        else if (gx==3 && gy==2) g32 = avg;
        else if (gx==0 && gy==3) g03 = avg;
        else if (gx==1 && gy==3) g13 = avg;
        else if (gx==2 && gy==3) g23 = avg;
        else                      g33 = avg;
      }
    }

    // Apply 2-level Haar on the 4x4 grid (same as level 2 code)
    // Level 1 horizontal
    vec3 r0_a0=(g00+g10)*0.5; vec3 r0_d0=(g00-g10)*0.5;
    vec3 r0_a1=(g20+g30)*0.5; vec3 r0_d1=(g20-g30)*0.5;
    vec3 r1_a0=(g01+g11)*0.5; vec3 r1_d0=(g01-g11)*0.5;
    vec3 r1_a1=(g21+g31)*0.5; vec3 r1_d1=(g21-g31)*0.5;
    vec3 r2_a0=(g02+g12)*0.5; vec3 r2_d0=(g02-g12)*0.5;
    vec3 r2_a1=(g22+g32)*0.5; vec3 r2_d1=(g22-g32)*0.5;
    vec3 r3_a0=(g03+g13)*0.5; vec3 r3_d0=(g03-g13)*0.5;
    vec3 r3_a1=(g23+g33)*0.5; vec3 r3_d1=(g23-g33)*0.5;

    // Level 1 vertical
    vec3 c_aa00=(r0_a0+r1_a0)*0.5; vec3 c_da00=(r0_a0-r1_a0)*0.5;
    vec3 c_aa10=(r0_a1+r1_a1)*0.5; vec3 c_da10=(r0_a1-r1_a1)*0.5;
    vec3 c_aa01=(r2_a0+r3_a0)*0.5; vec3 c_da01=(r2_a0-r3_a0)*0.5;
    vec3 c_aa11=(r2_a1+r3_a1)*0.5; vec3 c_da11=(r2_a1-r3_a1)*0.5;
    vec3 c_ad00=(r0_d0+r1_d0)*0.5; vec3 c_dd00=(r0_d0-r1_d0)*0.5;
    vec3 c_ad10=(r0_d1+r1_d1)*0.5; vec3 c_dd10=(r0_d1-r1_d1)*0.5;
    vec3 c_ad01=(r2_d0+r3_d0)*0.5; vec3 c_dd01=(r2_d0-r3_d0)*0.5;
    vec3 c_ad11=(r2_d1+r3_d1)*0.5; vec3 c_dd11=(r2_d1-r3_d1)*0.5;

    // Level 2 on LL
    vec3 ll2, lh2, hl2, hh2;
    haar2d_level1(c_aa00, c_aa10, c_aa01, c_aa11, ll2, lh2, hl2, hh2);

    // Apply gains
    ll2 *= gainLL; lh2 *= gainLH; hl2 *= gainHL; hh2 *= gainHH;
    c_ad00 *= gainLH; c_ad10 *= gainLH; c_ad01 *= gainLH; c_ad11 *= gainLH;
    c_da00 *= gainHL; c_da10 *= gainHL; c_da01 *= gainHL; c_da11 *= gainHL;
    c_dd00 *= gainHH; c_dd10 *= gainHH; c_dd01 *= gainHH; c_dd11 *= gainHH;

    // Quantize
    if (quantCoeffs > 0.5) {
      lh2 = quantize(lh2, quantCoeffs);
      hl2 = quantize(hl2, quantCoeffs);
      hh2 = quantize(hh2, quantCoeffs);
    }

    if (vizMode == 1) {
      // Subband viz based on grid position
      vec3 val = vec3(0.5);
      if (gridX < 2 && gridY < 2) {
        if (gridX==0 && gridY==0) val = ll2 + 0.5;
        else if (gridX==1 && gridY==0) val = lh2 + 0.5;
        else if (gridX==0 && gridY==1) val = hl2 + 0.5;
        else val = hh2 + 0.5;
      } else if (gridX >= 2) {
        val = vec3(abs(c_ad00.r + c_ad10.r) * 0.5) + 0.5;
      } else {
        val = vec3(abs(c_da00.r + c_da01.r) * 0.5) + 0.5;
      }
      result = val;
    } else if (vizMode == 2) {
      vec3 mag = abs(lh2) + abs(hl2) + abs(hh2);
      result = mag * 3.0;
    } else {
      // Reconstruct
      ihaar2d_level1(ll2, lh2, hl2, hh2, c_aa00, c_aa10, c_aa01, c_aa11);

      // Inverse level-1
      vec3 ir0_a0=c_aa00+c_da00; vec3 ir1_a0=c_aa00-c_da00;
      vec3 ir0_a1=c_aa10+c_da10; vec3 ir1_a1=c_aa10-c_da10;
      vec3 ir2_a0=c_aa01+c_da01; vec3 ir3_a0=c_aa01-c_da01;
      vec3 ir2_a1=c_aa11+c_da11; vec3 ir3_a1=c_aa11-c_da11;
      vec3 ir0_d0=c_ad00+c_dd00; vec3 ir1_d0=c_ad00-c_dd00;
      vec3 ir0_d1=c_ad10+c_dd10; vec3 ir1_d1=c_ad10-c_dd10;
      vec3 ir2_d0=c_ad01+c_dd01; vec3 ir3_d0=c_ad01-c_dd01;
      vec3 ir2_d1=c_ad11+c_dd11; vec3 ir3_d1=c_ad11-c_dd11;

      // Reconstruct 4x4 grid
      vec3 p[16];
      vec3 rg00=ir0_a0+ir0_d0; vec3 rg10=ir0_a0-ir0_d0;
      vec3 rg20=ir0_a1+ir0_d1; vec3 rg30=ir0_a1-ir0_d1;
      vec3 rg01=ir1_a0+ir1_d0; vec3 rg11=ir1_a0-ir1_d0;
      vec3 rg21=ir1_a1+ir1_d1; vec3 rg31=ir1_a1-ir1_d1;
      vec3 rg02=ir2_a0+ir2_d0; vec3 rg12=ir2_a0-ir2_d0;
      vec3 rg22=ir2_a1+ir2_d1; vec3 rg32=ir2_a1-ir2_d1;
      vec3 rg03=ir3_a0+ir3_d0; vec3 rg13=ir3_a0-ir3_d0;
      vec3 rg23=ir3_a1+ir3_d1; vec3 rg33=ir3_a1-ir3_d1;

      // Map pixel to its sub-block's reconstructed value
      if      (gridX==0 && gridY==0) result = rg00;
      else if (gridX==1 && gridY==0) result = rg10;
      else if (gridX==2 && gridY==0) result = rg20;
      else if (gridX==3 && gridY==0) result = rg30;
      else if (gridX==0 && gridY==1) result = rg01;
      else if (gridX==1 && gridY==1) result = rg11;
      else if (gridX==2 && gridY==1) result = rg21;
      else if (gridX==3 && gridY==1) result = rg31;
      else if (gridX==0 && gridY==2) result = rg02;
      else if (gridX==1 && gridY==2) result = rg12;
      else if (gridX==2 && gridY==2) result = rg22;
      else if (gridX==3 && gridY==2) result = rg32;
      else if (gridX==0 && gridY==3) result = rg03;
      else if (gridX==1 && gridY==3) result = rg13;
      else if (gridX==2 && gridY==3) result = rg23;
      else                            result = rg33;
    }
  }

  result = mix(rgb, result, intensity);
  gl_FragColor = vec4(result, original.a);
}
