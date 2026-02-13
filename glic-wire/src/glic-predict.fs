/*{
  "ISFVSN": "2",
  "VSN": "1.0",
  "DESCRIPTION": "GLIC Prediction Glitch — Predict pixel values from neighbors using codec algorithms. Visualize residuals or create glitch by mismatching encode/decode prediction.",
  "CREDIT": "GLIC port by Claude",
  "CATEGORIES": ["GLIC"],
  "INPUTS": [
    {
      "NAME": "inputImage",
      "TYPE": "image"
    },
    {
      "NAME": "predictionMode",
      "TYPE": "long",
      "VALUES": [0,1,2,3,4,5,6,7,8,9,10,11,12],
      "LABELS": ["Corner","Horizontal","Vertical","DC Average","Median","TrueMotion","Paeth","JPEG-LS","Average","Left Diagonal","H/V Select","Difference","Angle"],
      "DEFAULT": 6
    },
    {
      "NAME": "decodePrediction",
      "TYPE": "long",
      "VALUES": [0,1,2,3,4,5,6,7,8,9,10,11,12],
      "LABELS": ["Corner","Horizontal","Vertical","DC Average","Median","TrueMotion","Paeth","JPEG-LS","Average","Left Diagonal","H/V Select","Difference","Angle"],
      "DEFAULT": 6
    },
    {
      "NAME": "outputMode",
      "TYPE": "long",
      "VALUES": [0,1,2],
      "LABELS": ["Residuals","Prediction Only","Encode/Decode Mismatch"],
      "DEFAULT": 0
    },
    {
      "NAME": "residualGain",
      "TYPE": "float",
      "DEFAULT": 2.0,
      "MIN": 0.0,
      "MAX": 8.0
    },
    {
      "NAME": "residualOffset",
      "TYPE": "float",
      "DEFAULT": 0.5,
      "MIN": 0.0,
      "MAX": 1.0
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
// Neighbor sampling helpers
// ============================================================

vec2 px;  // pixel size in normalized coords

vec3 sampleAt(vec2 uv, float dx, float dy) {
  return IMG_NORM_PIXEL(inputImage, clamp(uv + vec2(dx, dy) * px, 0.0, 1.0)).rgb;
}

// left, top, corner (top-left diagonal) — the three key neighbors
vec3 getLeft(vec2 uv)   { return sampleAt(uv, -1.0,  0.0); }
vec3 getTop(vec2 uv)    { return sampleAt(uv,  0.0,  1.0); }
vec3 getCorner(vec2 uv) { return sampleAt(uv, -1.0,  1.0); }

// Additional neighbors for some modes
vec3 getRight(vec2 uv)      { return sampleAt(uv,  1.0,  0.0); }
vec3 getBottom(vec2 uv)     { return sampleAt(uv,  0.0, -1.0); }
vec3 getTopRight(vec2 uv)   { return sampleAt(uv,  1.0,  1.0); }
vec3 getBottomLeft(vec2 uv) { return sampleAt(uv, -1.0, -1.0); }

// ============================================================
// Prediction algorithms
// ============================================================

// Component-wise median of 3 values
vec3 median3(vec3 a, vec3 b, vec3 c) {
  return max(min(a, b), min(max(a, b), c));
}

// Paeth predictor (PNG): choose nearest of left, top, corner
vec3 paeth(vec3 left, vec3 top, vec3 corner) {
  vec3 p = left + top - corner;
  vec3 pa = abs(p - left);
  vec3 pb = abs(p - top);
  vec3 pc = abs(p - corner);
  vec3 result;
  result.r = (pa.r <= pb.r && pa.r <= pc.r) ? left.r : (pb.r <= pc.r ? top.r : corner.r);
  result.g = (pa.g <= pb.g && pa.g <= pc.g) ? left.g : (pb.g <= pc.g ? top.g : corner.g);
  result.b = (pa.b <= pb.b && pa.b <= pc.b) ? left.b : (pb.b <= pc.b ? top.b : corner.b);
  return result;
}

// JPEG-LS adaptive predictor
vec3 jpegls(vec3 left, vec3 top, vec3 corner) {
  vec3 result;
  // Per component: if corner >= max(left,top) -> min; if corner <= min(left,top) -> max; else left+top-corner
  result.r = (corner.r >= max(left.r, top.r)) ? min(left.r, top.r) : (corner.r <= min(left.r, top.r)) ? max(left.r, top.r) : left.r + top.r - corner.r;
  result.g = (corner.g >= max(left.g, top.g)) ? min(left.g, top.g) : (corner.g <= min(left.g, top.g)) ? max(left.g, top.g) : left.g + top.g - corner.g;
  result.b = (corner.b >= max(left.b, top.b)) ? min(left.b, top.b) : (corner.b <= min(left.b, top.b)) ? max(left.b, top.b) : left.b + top.b - corner.b;
  return result;
}

vec3 predict(int mode, vec2 uv) {
  vec3 left = getLeft(uv);
  vec3 top = getTop(uv);
  vec3 corner = getCorner(uv);

  // 0: Corner — predict from top-left diagonal
  if (mode == 0) return corner;

  // 1: Horizontal — propagate left neighbor
  if (mode == 1) return left;

  // 2: Vertical — propagate top neighbor
  if (mode == 2) return top;

  // 3: DC Average — average of top and left boundary samples
  if (mode == 3) {
    // Sample a few pixels along top and left edges near this pixel
    vec3 avg = vec3(0.0);
    avg += sampleAt(uv, -1.0, 0.0);
    avg += sampleAt(uv, -2.0, 0.0);
    avg += sampleAt(uv,  0.0, 1.0);
    avg += sampleAt(uv,  0.0, 2.0);
    avg += sampleAt(uv, -1.0, 1.0);
    avg += sampleAt(uv, -2.0, 1.0);
    return avg / 6.0;
  }

  // 4: Median — median of left, top, corner
  if (mode == 4) return median3(left, top, corner);

  // 5: TrueMotion — gradient predictor: left + top - corner
  if (mode == 5) return left + top - corner;

  // 6: Paeth — PNG predictor
  if (mode == 6) return paeth(left, top, corner);

  // 7: JPEG-LS — adaptive min/max predictor
  if (mode == 7) return jpegls(left, top, corner);

  // 8: Average — (left + top) / 2
  if (mode == 8) return (left + top) * 0.5;

  // 9: Left Diagonal — weighted interpolation along diagonal
  if (mode == 9) {
    vec3 bl = getBottomLeft(uv);
    return (left * 0.5 + corner * 0.25 + bl * 0.25);
  }

  // 10: H/V Select — choose horizontal or vertical by pixel position
  if (mode == 10) {
    vec2 pixelPos = uv * RENDERSIZE;
    return (mod(pixelPos.x + pixelPos.y, 2.0) < 1.0) ? left : top;
  }

  // 11: Difference — second-order: 2*left - corner
  if (mode == 11) return 2.0 * left - corner;

  // 12: Angle — gradient-based directional prediction
  if (mode == 12) {
    vec3 right = getRight(uv);
    vec3 bottom = getBottom(uv);
    // Compute horizontal and vertical gradients
    vec3 gx = right - left;
    vec3 gy = top - bottom;
    // Use gradient magnitude to weight between horizontal and vertical prediction
    float hStrength = dot(abs(gy), vec3(1.0));  // strong vertical gradient -> use horizontal prediction
    float vStrength = dot(abs(gx), vec3(1.0));  // strong horizontal gradient -> use vertical prediction
    float total = hStrength + vStrength + 0.001;
    return left * (hStrength / total) + top * (vStrength / total);
  }

  return left;
}

// ============================================================
// Main
// ============================================================

void main() {
  vec2 uv = isf_FragNormCoord;
  px = 1.0 / RENDERSIZE;

  vec4 original = IMG_NORM_PIXEL(inputImage, uv);
  vec3 rgb = original.rgb;

  if (outputMode == 0) {
    // Mode 0: Visualize residuals
    vec3 predicted = predict(predictionMode, uv);
    vec3 residual = (rgb - predicted) * residualGain + residualOffset;
    vec3 result = mix(rgb, residual, intensity);
    gl_FragColor = vec4(result, original.a);

  } else if (outputMode == 1) {
    // Mode 1: Show prediction only
    vec3 predicted = predict(predictionMode, uv);
    vec3 result = mix(rgb, predicted, intensity);
    gl_FragColor = vec4(result, original.a);

  } else {
    // Mode 2: Encode/Decode mismatch
    // Encode: subtract prediction A to get residual
    vec3 encodePred = predict(predictionMode, uv);
    vec3 residual = rgb - encodePred;

    // Scale residual
    residual *= residualGain;

    // Decode: add back prediction B (different from A = glitch)
    vec3 decodePred = predict(decodePrediction, uv);
    vec3 reconstructed = decodePred + residual;

    vec3 result = mix(rgb, reconstructed, intensity);
    gl_FragColor = vec4(result, original.a);
  }
}
