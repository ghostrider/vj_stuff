/*{
  "ISFVSN": "2",
  "VSN": "1.0",
  "DESCRIPTION": "GLIC Segmentation — Adaptive block mosaic that approximates GLIC's quadtree segmentation. Areas with high detail keep small blocks, flat areas get large blocks.",
  "CREDIT": "GLIC port by Claude",
  "CATEGORIES": ["GLIC"],
  "INPUTS": [
    {
      "NAME": "inputImage",
      "TYPE": "image"
    },
    {
      "NAME": "minBlockPow",
      "TYPE": "long",
      "VALUES": [1,2,3,4,5],
      "LABELS": ["2px","4px","8px","16px","32px"],
      "DEFAULT": 1
    },
    {
      "NAME": "maxBlockPow",
      "TYPE": "long",
      "VALUES": [3,4,5,6,7],
      "LABELS": ["8px","16px","32px","64px","128px"],
      "DEFAULT": 5
    },
    {
      "NAME": "threshold",
      "TYPE": "float",
      "DEFAULT": 0.1,
      "MIN": 0.0,
      "MAX": 1.0
    },
    {
      "NAME": "showGrid",
      "TYPE": "bool",
      "DEFAULT": false
    },
    {
      "NAME": "gridColor",
      "TYPE": "color",
      "DEFAULT": [1.0, 1.0, 1.0, 0.5]
    },
    {
      "NAME": "colorMode",
      "TYPE": "long",
      "VALUES": [0,1,2],
      "LABELS": ["Block Average","Block Center","Variance Map"],
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
// Adaptive quadtree-like segmentation
//
// Strategy: For each pixel, test variance at progressively larger
// block sizes. Start at max block. If variance is above threshold,
// subdivide (use smaller block). This approximates GLIC's quadtree
// without needing actual recursive data structures.
// ============================================================

// Fast luminance
float luma(vec3 c) {
  return dot(c, vec3(0.299, 0.587, 0.114));
}

// Sample pixel at absolute pixel offset
vec3 samplePx(vec2 pixelCoord) {
  return IMG_NORM_PIXEL(inputImage, pixelCoord / RENDERSIZE).rgb;
}

// Compute local variance in a block around a center point
// Uses a sparse sampling pattern for performance
float blockVariance(vec2 blockOrigin, float blockSize) {
  float halfBlock = blockSize * 0.5;
  vec2 center = blockOrigin + halfBlock;

  // Sample 9 points in a grid pattern within the block
  float sum = 0.0;
  float sumSq = 0.0;
  float n = 0.0;

  for (float dy = 0.0; dy < 3.0; dy += 1.0) {
    for (float dx = 0.0; dx < 3.0; dx += 1.0) {
      vec2 p = blockOrigin + vec2(dx, dy) * (blockSize / 2.0);
      p = clamp(p, vec2(0.0), RENDERSIZE - 1.0);
      float l = luma(samplePx(p));
      sum += l;
      sumSq += l * l;
      n += 1.0;
    }
  }

  float mean = sum / n;
  return sumSq / n - mean * mean;
}

// Compute the block average color
vec3 blockAverage(vec2 blockOrigin, float blockSize) {
  vec3 sum = vec3(0.0);
  float n = 0.0;

  // Adaptive sampling density based on block size
  float step = max(1.0, blockSize / 4.0);

  for (float dy = 0.0; dy < blockSize; dy += step) {
    for (float dx = 0.0; dx < blockSize; dx += step) {
      vec2 p = blockOrigin + vec2(dx, dy);
      p = clamp(p, vec2(0.0), RENDERSIZE - 1.0);
      sum += samplePx(p);
      n += 1.0;
    }
  }

  return sum / max(n, 1.0);
}

void main() {
  vec2 uv = isf_FragNormCoord;
  vec4 original = IMG_NORM_PIXEL(inputImage, uv);
  vec3 rgb = original.rgb;

  vec2 pixelPos = uv * RENDERSIZE;

  float minBlock = pow(2.0, float(minBlockPow));
  float maxBlock = pow(2.0, float(maxBlockPow));

  // Ensure min <= max
  float actualMin = min(minBlock, maxBlock);
  float actualMax = max(minBlock, maxBlock);

  // Find appropriate block size for this pixel using top-down subdivision
  float chosenSize = actualMax;
  float varianceScale = threshold * threshold; // square for perceptual scaling

  // Start from largest block, subdivide if variance is high
  for (float testSize = actualMax; testSize > actualMin; testSize *= 0.5) {
    vec2 blockOrigin = floor(pixelPos / testSize) * testSize;
    float var = blockVariance(blockOrigin, testSize);

    if (var > varianceScale) {
      // High variance: subdivide (use smaller block)
      chosenSize = testSize * 0.5;
    } else {
      // Low variance: this block size is fine
      chosenSize = testSize;
      break;
    }
  }

  chosenSize = clamp(chosenSize, actualMin, actualMax);

  // Compute block origin for chosen size
  vec2 blockOrigin = floor(pixelPos / chosenSize) * chosenSize;

  // Determine output color based on mode
  vec3 blockColor;
  float variance = blockVariance(blockOrigin, chosenSize);

  if (colorMode == 0) {
    // Block average
    blockColor = blockAverage(blockOrigin, chosenSize);
  } else if (colorMode == 1) {
    // Block center pixel
    vec2 center = blockOrigin + chosenSize * 0.5;
    blockColor = samplePx(clamp(center, vec2(0.0), RENDERSIZE - 1.0));
  } else {
    // Variance visualization: map variance to color
    float v = sqrt(variance) * 5.0;
    blockColor = vec3(v, v * 0.5, 1.0 - v);
  }

  // Grid overlay
  if (showGrid) {
    vec2 relPos = mod(pixelPos, chosenSize);
    float edgeDist = min(min(relPos.x, relPos.y), min(chosenSize - relPos.x, chosenSize - relPos.y));
    if (edgeDist < 1.0) {
      blockColor = mix(blockColor, gridColor.rgb, gridColor.a);
    }
  }

  vec3 result = mix(rgb, blockColor, intensity);
  gl_FragColor = vec4(result, original.a);
}
