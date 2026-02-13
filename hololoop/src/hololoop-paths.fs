/*{
  "ISFVSN": "2",
  "VSN": "1.0",
  "DESCRIPTION": "Hololoop Paths — Golden-ratio rectangle mosaic oriented along image gradients. Rectangles sized by noise and local variance, rendered with the original's dark-fill/light-stroke style. 7 color palettes from the original. Port of Hololoop Mode 2.",
  "CREDIT": "Hololoop by wabisabit, ISF port by Claude",
  "CATEGORIES": ["Hololoop"],
  "INPUTS": [
    {
      "NAME": "inputImage",
      "TYPE": "image"
    },
    {
      "NAME": "cellSize",
      "TYPE": "float",
      "DEFAULT": 40.0,
      "MIN": 10.0,
      "MAX": 120.0
    },
    {
      "NAME": "sizeVariation",
      "TYPE": "float",
      "DEFAULT": 0.5,
      "MIN": 0.0,
      "MAX": 1.0
    },
    {
      "NAME": "strokeWidth",
      "TYPE": "float",
      "DEFAULT": 2.0,
      "MIN": 0.5,
      "MAX": 6.0
    },
    {
      "NAME": "orientByImage",
      "TYPE": "bool",
      "DEFAULT": true
    },
    {
      "NAME": "colorSwitch",
      "TYPE": "bool",
      "DEFAULT": false
    },
    {
      "NAME": "palette",
      "TYPE": "long",
      "VALUES": [0, 1, 2, 3, 4, 5, 6],
      "LABELS": ["Warm Sand", "Coral Garden", "Watermelon", "Mint Noir", "Autumn", "Fiesta", "Ocean Neon"],
      "DEFAULT": 0
    },
    {
      "NAME": "audioReactivity",
      "TYPE": "float",
      "DEFAULT": 0.0,
      "MIN": 0.0,
      "MAX": 1.0
    },
    {
      "NAME": "psychoSwitch",
      "TYPE": "bool",
      "DEFAULT": false
    },
    {
      "NAME": "noiseScale",
      "TYPE": "float",
      "DEFAULT": 0.3,
      "MIN": 0.0,
      "MAX": 1.0
    },
    {
      "NAME": "fillOpacity",
      "TYPE": "float",
      "DEFAULT": 0.0,
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

// --- Hash / noise utilities ---

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

vec2 hash2(vec2 p) {
  return vec2(
    fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453),
    fract(sin(dot(p, vec2(269.5, 183.3))) * 43758.5453)
  );
}

// Simple value noise
float vnoise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  f = f * f * (3.0 - 2.0 * f); // smoothstep
  float a = hash(i);
  float b = hash(i + vec2(1.0, 0.0));
  float c = hash(i + vec2(0.0, 1.0));
  float d = hash(i + vec2(1.0, 1.0));
  return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

float luminance(vec3 c) {
  return dot(c, vec3(0.2126, 0.7152, 0.0722));
}

// --- 7 original Hololoop palettes ---
// Each palette has 5 colors, encoded as vec3 RGB [0-1]

vec3 getPaletteColor(int pal, int idx) {
  // Palette 0: Warm Sand { #BBBB88, #CCC68D, #EEDD99, #EEC290, #EEAA88 }
  if (pal == 0) {
    if (idx == 0) return vec3(0.733, 0.733, 0.533);
    if (idx == 1) return vec3(0.800, 0.776, 0.553);
    if (idx == 2) return vec3(0.933, 0.867, 0.600);
    if (idx == 3) return vec3(0.933, 0.761, 0.565);
    return vec3(0.933, 0.667, 0.533);
  }
  // Palette 1: Coral Garden { #FF4242, #F4FAD2, #D4EE5E, #E1EDB9, #F0F2EB }
  if (pal == 1) {
    if (idx == 0) return vec3(1.000, 0.259, 0.259);
    if (idx == 1) return vec3(0.957, 0.980, 0.824);
    if (idx == 2) return vec3(0.831, 0.933, 0.369);
    if (idx == 3) return vec3(0.882, 0.929, 0.725);
    return vec3(0.941, 0.949, 0.922);
  }
  // Palette 2: Watermelon { #D1F2A5, #EFFAB4, #FFC48C, #FF9F80, #F56991 }
  if (pal == 2) {
    if (idx == 0) return vec3(0.820, 0.949, 0.647);
    if (idx == 1) return vec3(0.937, 0.980, 0.706);
    if (idx == 2) return vec3(1.000, 0.769, 0.549);
    if (idx == 3) return vec3(1.000, 0.624, 0.502);
    return vec3(0.961, 0.412, 0.569);
  }
  // Palette 3: Mint Noir { #CFFFDD, #B4DEC1, #5C5863, #A85163, #FF1F4C }
  if (pal == 3) {
    if (idx == 0) return vec3(0.812, 1.000, 0.867);
    if (idx == 1) return vec3(0.706, 0.871, 0.757);
    if (idx == 2) return vec3(0.361, 0.345, 0.388);
    if (idx == 3) return vec3(0.659, 0.318, 0.388);
    return vec3(1.000, 0.122, 0.298);
  }
  // Palette 4: Autumn { #B3CC57, #ECF081, #FFBE40, #EF746F, #AB3E5B }
  if (pal == 4) {
    if (idx == 0) return vec3(0.702, 0.800, 0.341);
    if (idx == 1) return vec3(0.925, 0.941, 0.506);
    if (idx == 2) return vec3(1.000, 0.745, 0.251);
    if (idx == 3) return vec3(0.937, 0.455, 0.435);
    return vec3(0.671, 0.243, 0.357);
  }
  // Palette 5: Fiesta { #CC0C39, #E6781E, #C8CF02, #F8FCC1, #1693A7 }
  if (pal == 5) {
    if (idx == 0) return vec3(0.800, 0.047, 0.224);
    if (idx == 1) return vec3(0.902, 0.471, 0.118);
    if (idx == 2) return vec3(0.784, 0.812, 0.008);
    if (idx == 3) return vec3(0.973, 0.988, 0.757);
    return vec3(0.086, 0.576, 0.655);
  }
  // Palette 6: Ocean Neon { #1693A5, #02AAB0, #00CDAC, #7FFF24, #C3FF68 }
  if (idx == 0) return vec3(0.086, 0.576, 0.647);
  if (idx == 1) return vec3(0.008, 0.667, 0.690);
  if (idx == 2) return vec3(0.000, 0.804, 0.675);
  if (idx == 3) return vec3(0.498, 1.000, 0.141);
  return vec3(0.765, 1.000, 0.408);
}

// Compute image gradient at uv for rectangle orientation
vec2 imageGradient(vec2 uv) {
  vec2 texel = 1.0 / RENDERSIZE;
  float dx = 3.0; // sample offset in pixels
  float l = luminance(IMG_NORM_PIXEL(inputImage, uv + vec2(-dx * texel.x, 0.0)).rgb);
  float r = luminance(IMG_NORM_PIXEL(inputImage, uv + vec2( dx * texel.x, 0.0)).rgb);
  float d = luminance(IMG_NORM_PIXEL(inputImage, uv + vec2(0.0, -dx * texel.y)).rgb);
  float u = luminance(IMG_NORM_PIXEL(inputImage, uv + vec2(0.0,  dx * texel.y)).rgb);
  return vec2(r - l, u - d);
}

void main() {
  vec2 uv = isf_FragNormCoord;
  vec2 pixel = uv * RENDERSIZE;
  vec4 original = IMG_NORM_PIXEL(inputImage, uv);

  float cs = cellSize;

  // Simulated audio amplitude
  float simAmp = luminance(IMG_NORM_PIXEL(inputImage, vec2(0.5)).rgb) * audioReactivity;

  // Which grid cell is this pixel in?
  vec2 gridIdx = floor(pixel / cs);
  vec2 cellCenter = (gridIdx + 0.5) * cs;
  vec2 cellUV = clamp(cellCenter / RENDERSIZE, 0.0, 1.0);

  // Jitter cell center slightly for organic feel
  vec2 jitter = (hash2(gridIdx * 5.3) - 0.5) * cs * 0.2 * sizeVariation;
  cellCenter += jitter;

  // Sample image at cell center
  vec3 cellColor = IMG_NORM_PIXEL(inputImage, clamp(cellCenter / RENDERSIZE, 0.0, 1.0)).rgb;
  float cellLum = luminance(cellColor);

  // Rectangle orientation: from image gradient or noise
  float angle = 0.0;
  if (orientByImage) {
    vec2 grad = imageGradient(cellUV);
    // Orient perpendicular to gradient (along edges)
    angle = atan(grad.y, grad.x) + 1.5707963;
  }
  // Add noise-based variation
  angle += (vnoise(gridIdx * noiseScale * 3.0 + TIME * 0.05) - 0.5) * noiseScale * 3.14159;

  // Rectangle dimensions: golden ratio (w = h * 1.618)
  float baseSize = cs * (0.4 + sizeVariation * vnoise(gridIdx * 0.5 + 10.0) * 0.6);
  // Audio-reactive scaling (matches original: amp * audioScale * 1.618)
  baseSize *= 1.0 + simAmp * 1.618;
  float rectH = baseSize;
  float rectW = rectH * 1.618;

  // Transform pixel into rectangle's local space
  vec2 local = pixel - cellCenter;
  float ca = cos(angle);
  float sa = sin(angle);
  vec2 rotated = vec2(local.x * ca + local.y * sa, -local.x * sa + local.y * ca);

  // Check if pixel is inside the rectangle
  float halfW = rectW * 0.5;
  float halfH = rectH * 0.5;
  bool inside = abs(rotated.x) < halfW && abs(rotated.y) < halfH;

  vec3 result = original.rgb;

  if (inside) {
    // Distance to nearest edge (for stroke detection)
    float edgeDistX = halfW - abs(rotated.x);
    float edgeDistY = halfH - abs(rotated.y);
    float edgeDist = min(edgeDistX, edgeDistY);

    // Stroke: white border
    float strokeMask = smoothstep(strokeWidth + 0.5, strokeWidth - 0.5, edgeDist);

    // Fill color
    vec3 fill = vec3(0.0); // Black fill (original default)

    if (colorSwitch) {
      // Palette coloring: index from noise (matches original: noise(ampSum + x + y))
      float noiseVal = vnoise(gridIdx * 0.7 + TIME * 0.1);
      int colorIdx = int(noiseVal * 5.0);
      colorIdx = clamp(colorIdx, 0, 4);
      fill = getPaletteColor(palette, colorIdx);
    }

    if (psychoSwitch) {
      // Random colors per cell, changing over time
      fill = vec3(
        hash(gridIdx * 3.0 + floor(TIME * 6.0)),
        hash(gridIdx * 5.0 + floor(TIME * 6.0) + 1.0),
        hash(gridIdx * 7.0 + floor(TIME * 6.0) + 2.0)
      );
    }

    // Blend fill with source image
    fill = mix(fill, cellColor, fillOpacity);

    // Composite: fill + stroke
    vec3 stroke = vec3(1.0); // White stroke
    result = mix(fill, stroke, strokeMask);
  }

  gl_FragColor = vec4(mix(original.rgb, result, intensity), original.a);
}
