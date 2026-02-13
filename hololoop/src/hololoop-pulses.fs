/*{
  "ISFVSN": "2",
  "VSN": "1.0",
  "DESCRIPTION": "Hololoop Pulses — Expanding concentric circle pulses emanating from a grid of points. Pulse activity driven by input image brightness. Circles expand and fade over time with per-ring random opacity. Port of Hololoop Mode 3.",
  "CREDIT": "Hololoop by wabisabit, ISF port by Claude",
  "CATEGORIES": ["Hololoop"],
  "INPUTS": [
    {
      "NAME": "inputImage",
      "TYPE": "image"
    },
    {
      "NAME": "gridSize",
      "TYPE": "float",
      "DEFAULT": 100.0,
      "MIN": 30.0,
      "MAX": 250.0
    },
    {
      "NAME": "maxPulses",
      "TYPE": "long",
      "VALUES": [3, 5, 8, 10, 15],
      "LABELS": ["3", "5", "8", "10", "15"],
      "DEFAULT": 8
    },
    {
      "NAME": "expansionSpeed",
      "TYPE": "float",
      "DEFAULT": 0.5,
      "MIN": 0.1,
      "MAX": 2.0
    },
    {
      "NAME": "expansionRange",
      "TYPE": "float",
      "DEFAULT": 1.5,
      "MIN": 0.5,
      "MAX": 4.0
    },
    {
      "NAME": "ringWidth",
      "TYPE": "float",
      "DEFAULT": 2.0,
      "MIN": 0.5,
      "MAX": 8.0
    },
    {
      "NAME": "luminanceThreshold",
      "TYPE": "float",
      "DEFAULT": 0.2,
      "MIN": 0.0,
      "MAX": 1.0
    },
    {
      "NAME": "psychoSwitch",
      "TYPE": "bool",
      "DEFAULT": false
    },
    {
      "NAME": "showImage",
      "TYPE": "float",
      "DEFAULT": 0.2,
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

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

float hash3(vec3 p) {
  return fract(sin(dot(p, vec3(127.1, 311.7, 74.7))) * 43758.5453123);
}

float luminance(vec3 c) {
  return dot(c, vec3(0.2126, 0.7152, 0.0722));
}

void main() {
  vec2 uv = isf_FragNormCoord;
  vec2 pixel = uv * RENDERSIZE;
  vec4 original = IMG_NORM_PIXEL(inputImage, uv);

  float cs = gridSize;
  vec2 gridPos = floor(pixel / cs);

  float pulseAlpha = 0.0;
  vec3 pulseColor = vec3(1.0);

  // Check neighboring grid cells (wider range since pulses expand)
  int searchRange = int(ceil(expansionRange)) + 1;
  // Clamp search range for performance
  if (searchRange > 4) searchRange = 4;

  for (int dx = -4; dx <= 4; dx++) {
    if (dx < -searchRange || dx > searchRange) continue;
    for (int dy = -4; dy <= 4; dy++) {
      if (dy < -searchRange || dy > searchRange) continue;

      vec2 nodeGrid = gridPos + vec2(float(dx), float(dy));
      // Node center is offset to cell center (matches original: x*w + w/2)
      vec2 nodeCenter = (nodeGrid + 0.5) * cs;
      vec2 nodeUV = clamp(nodeCenter / RENDERSIZE, 0.0, 1.0);

      // Sample image at node to determine if pulse is active
      float nodeLum = luminance(IMG_NORM_PIXEL(inputImage, nodeUV).rgb);

      // Only pulse at sufficiently bright locations
      if (nodeLum < luminanceThreshold) continue;

      float dist = length(pixel - nodeCenter);

      // Number of pulse rings at this node (original: random 1-15 per node)
      int nPulses = int(hash(nodeGrid * 3.7) * float(maxPulses)) + 1;

      for (int p = 0; p < 15; p++) {
        if (p >= nPulses) break;

        // Each pulse has its own period and phase
        // Original: pulseWidths expand by 0.1 + random(5) per frame
        // Original: pulseOpacities decrease by 5 - random(1) per frame
        // We simulate this with time-based animation
        float period = 1.5 + hash(nodeGrid * 7.3 + float(p) * 13.1) * 3.0;
        period /= expansionSpeed;
        float phase = hash(nodeGrid * 11.7 + float(p) * 17.3) * period;

        float t = mod(TIME + phase, period) / period; // 0 -> 1 over one cycle

        // Pulse is active when driven by image brightness
        // Higher brightness = more frequent pulses (shorter effective period)
        float activationChance = nodeLum;
        float cycleHash = hash(nodeGrid * 19.3 + float(p) + floor((TIME + phase) / period));
        if (cycleHash > activationChance) continue;

        // Expanding radius (original: width += 0.1 + random(5))
        float maxRadius = cs * expansionRange;
        float radius = t * maxRadius;

        // Fading opacity (original: opacity -= 5 - random(1), starts at random(255))
        float startOpacity = hash3(vec3(nodeGrid, float(p) + 0.5)) * 1.0;
        float opacity = startOpacity * (1.0 - t);
        opacity = max(opacity, 0.0);

        // Ring rendering
        float rw = ringWidth;
        // Original psycho mode: strokeWeight(5 / pulseWidths[i])
        if (psychoSwitch) {
          rw = 5.0 / max(t * 10.0, 0.5);
          rw = clamp(rw, 1.0, 10.0);
        }

        float ring = smoothstep(rw + 0.5, 0.0, abs(dist - radius));
        float alpha = ring * opacity;

        if (psychoSwitch) {
          // In psycho mode, translate z = random(-100, 100) creates depth jitter
          // Simulate as brightness/scale variation
          float zJitter = hash3(vec3(nodeGrid, float(p) + floor(TIME * 10.0))) * 2.0 - 1.0;
          alpha *= 0.5 + 0.5 * abs(zJitter);
        }

        pulseAlpha = max(pulseAlpha, alpha);
      }
    }
  }

  // Composite
  vec3 base = original.rgb * showImage;
  vec3 result = base + vec3(1.0) * pulseAlpha;

  gl_FragColor = vec4(mix(original.rgb, result, intensity), original.a);
}
