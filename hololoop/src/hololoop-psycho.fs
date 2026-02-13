/*{
  "ISFVSN": "2",
  "VSN": "1.0",
  "DESCRIPTION": "Hololoop Psycho — Psychedelic color and displacement effect. Random per-pixel color cycling, Z-depth simulation via displacement, and amplitude-driven chaos. Combines the 'p' psycho mode and 7 color palettes from the original. Port of Hololoop's psychedelic mode.",
  "CREDIT": "Hololoop by wabisabit, ISF port by Claude",
  "CATEGORIES": ["Hololoop"],
  "INPUTS": [
    {
      "NAME": "inputImage",
      "TYPE": "image"
    },
    {
      "NAME": "colorChaos",
      "TYPE": "float",
      "DEFAULT": 0.5,
      "MIN": 0.0,
      "MAX": 1.0
    },
    {
      "NAME": "displacement",
      "TYPE": "float",
      "DEFAULT": 0.3,
      "MIN": 0.0,
      "MAX": 1.0
    },
    {
      "NAME": "flickerRate",
      "TYPE": "float",
      "DEFAULT": 0.5,
      "MIN": 0.0,
      "MAX": 1.0
    },
    {
      "NAME": "colorSwitch",
      "TYPE": "bool",
      "DEFAULT": true
    },
    {
      "NAME": "palette",
      "TYPE": "long",
      "VALUES": [0, 1, 2, 3, 4, 5, 6],
      "LABELS": ["Warm Sand", "Coral Garden", "Watermelon", "Mint Noir", "Autumn", "Fiesta", "Ocean Neon"],
      "DEFAULT": 5
    },
    {
      "NAME": "audioReactivity",
      "TYPE": "float",
      "DEFAULT": 0.0,
      "MIN": 0.0,
      "MAX": 1.0
    },
    {
      "NAME": "zDepthEffect",
      "TYPE": "float",
      "DEFAULT": 0.3,
      "MIN": 0.0,
      "MAX": 1.0
    },
    {
      "NAME": "blockSize",
      "TYPE": "float",
      "DEFAULT": 8.0,
      "MIN": 1.0,
      "MAX": 50.0
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

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

vec3 hash3v(vec2 p) {
  return vec3(
    fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453),
    fract(sin(dot(p, vec2(269.5, 183.3))) * 43758.5453),
    fract(sin(dot(p, vec2(419.2, 371.9))) * 43758.5453)
  );
}

float vnoise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  f = f * f * (3.0 - 2.0 * f);
  float a = hash(i);
  float b = hash(i + vec2(1.0, 0.0));
  float c = hash(i + vec2(0.0, 1.0));
  float d = hash(i + vec2(1.0, 1.0));
  return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

float luminance(vec3 c) {
  return dot(c, vec3(0.2126, 0.7152, 0.0722));
}

// 7 original Hololoop palettes
vec3 getPaletteColor(int pal, int idx) {
  if (pal == 0) {
    if (idx == 0) return vec3(0.733, 0.733, 0.533);
    if (idx == 1) return vec3(0.800, 0.776, 0.553);
    if (idx == 2) return vec3(0.933, 0.867, 0.600);
    if (idx == 3) return vec3(0.933, 0.761, 0.565);
    return vec3(0.933, 0.667, 0.533);
  }
  if (pal == 1) {
    if (idx == 0) return vec3(1.000, 0.259, 0.259);
    if (idx == 1) return vec3(0.957, 0.980, 0.824);
    if (idx == 2) return vec3(0.831, 0.933, 0.369);
    if (idx == 3) return vec3(0.882, 0.929, 0.725);
    return vec3(0.941, 0.949, 0.922);
  }
  if (pal == 2) {
    if (idx == 0) return vec3(0.820, 0.949, 0.647);
    if (idx == 1) return vec3(0.937, 0.980, 0.706);
    if (idx == 2) return vec3(1.000, 0.769, 0.549);
    if (idx == 3) return vec3(1.000, 0.624, 0.502);
    return vec3(0.961, 0.412, 0.569);
  }
  if (pal == 3) {
    if (idx == 0) return vec3(0.812, 1.000, 0.867);
    if (idx == 1) return vec3(0.706, 0.871, 0.757);
    if (idx == 2) return vec3(0.361, 0.345, 0.388);
    if (idx == 3) return vec3(0.659, 0.318, 0.388);
    return vec3(1.000, 0.122, 0.298);
  }
  if (pal == 4) {
    if (idx == 0) return vec3(0.702, 0.800, 0.341);
    if (idx == 1) return vec3(0.925, 0.941, 0.506);
    if (idx == 2) return vec3(1.000, 0.745, 0.251);
    if (idx == 3) return vec3(0.937, 0.455, 0.435);
    return vec3(0.671, 0.243, 0.357);
  }
  if (pal == 5) {
    if (idx == 0) return vec3(0.800, 0.047, 0.224);
    if (idx == 1) return vec3(0.902, 0.471, 0.118);
    if (idx == 2) return vec3(0.784, 0.812, 0.008);
    if (idx == 3) return vec3(0.973, 0.988, 0.757);
    return vec3(0.086, 0.576, 0.655);
  }
  if (idx == 0) return vec3(0.086, 0.576, 0.647);
  if (idx == 1) return vec3(0.008, 0.667, 0.690);
  if (idx == 2) return vec3(0.000, 0.804, 0.675);
  if (idx == 3) return vec3(0.498, 1.000, 0.141);
  return vec3(0.765, 1.000, 0.408);
}

void main() {
  vec2 uv = isf_FragNormCoord;
  vec2 pixel = uv * RENDERSIZE;
  vec4 original = IMG_NORM_PIXEL(inputImage, uv);

  // Simulated audio amplitude
  float simAmp = luminance(original.rgb) * audioReactivity;

  // Time quantized by flicker rate for frame-stepping effect
  float flickerTime = floor(TIME * (2.0 + flickerRate * 30.0));

  // --- Z-depth displacement ---
  // Original psycho mode: position.z += randomGaussian() * 5
  // Simulated as UV displacement based on noise
  vec2 displaceUV = uv;
  if (displacement > 0.0) {
    // Block-based displacement (grouped by blockSize for coherent movement)
    vec2 blockIdx = floor(pixel / blockSize);
    vec2 noiseOffset = vec2(
      hash(blockIdx * 3.1 + flickerTime * 0.7) - 0.5,
      hash(blockIdx * 5.3 + flickerTime * 0.7 + 1.0) - 0.5
    );
    // Gaussian-like distribution: multiply two uniform randoms
    float gaussApprox = (hash(blockIdx + flickerTime) - 0.5) *
                        (hash(blockIdx * 2.0 + flickerTime + 3.0) - 0.5) * 4.0;
    noiseOffset *= gaussApprox;
    displaceUV = uv + noiseOffset * displacement * 0.1;
    displaceUV = clamp(displaceUV, 0.0, 1.0);
  }

  vec4 displaced = IMG_NORM_PIXEL(inputImage, displaceUV);
  vec3 result = displaced.rgb;

  // --- Color chaos ---
  // Original: fill(random(255), random(255), random(255), random(255))
  if (colorChaos > 0.0) {
    vec2 blockIdx = floor(pixel / blockSize);
    vec3 randomColor = hash3v(blockIdx * 7.0 + flickerTime);

    // Mix random color with displaced image based on chaos amount
    result = mix(result, randomColor, colorChaos);
  }

  // --- Palette coloring ---
  // Original colorSwitch: fill(currentPalette[noise(ampSum + x + y) * len])
  if (colorSwitch) {
    vec2 blockIdx = floor(pixel / blockSize);
    float noiseVal = vnoise(blockIdx * 0.3 + TIME * 0.2);
    // Combine with luminance for image-reactive palette mapping
    float lum = luminance(displaced.rgb);
    float palIdx = noiseVal * 0.7 + lum * 0.3;
    int colorIdx = int(palIdx * 5.0);
    colorIdx = clamp(colorIdx, 0, 4);
    vec3 palColor = getPaletteColor(palette, colorIdx);

    // Blend palette color based on colorChaos and luminance
    float palBlend = colorChaos * 0.5 + 0.2;
    result = mix(result, palColor * (0.5 + lum * 0.5), palBlend);
  }

  // --- Audio-reactive pulse (simulated amplitude spikes) ---
  if (audioReactivity > 0.0) {
    // Original: rayV.mult(1 + amp) on random chance
    float spike = step(0.8, hash(floor(pixel / blockSize) + flickerTime * 0.3));
    result *= 1.0 + spike * simAmp * 2.0;
  }

  // --- Z-depth brightness variation ---
  // Original: particles at varying Z positions have varying brightness
  // stroke(255 + position.z / 10) where z goes negative
  if (zDepthEffect > 0.0) {
    vec2 blockIdx = floor(pixel / blockSize);
    float z = (hash(blockIdx * 9.1 + flickerTime) - 0.5) * 2.0;
    float brightness = 1.0 + z * zDepthEffect * 0.3;
    result *= brightness;
  }

  result = clamp(result, 0.0, 1.0);

  gl_FragColor = vec4(mix(original.rgb, result, intensity), original.a);
}
