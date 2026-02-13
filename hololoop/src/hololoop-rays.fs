/*{
  "ISFVSN": "2",
  "VSN": "1.0",
  "DESCRIPTION": "Hololoop Rays — Grid of nodes emitting rays toward bright regions of the input image. Ray count and length driven by local luminance. Animated rotation with audio-reactive growth. Port of Hololoop Mode 1.",
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
      "DEFAULT": 80.0,
      "MIN": 20.0,
      "MAX": 200.0
    },
    {
      "NAME": "maxRays",
      "TYPE": "long",
      "VALUES": [4, 8, 12, 16, 24, 32],
      "LABELS": ["4", "8", "12", "16", "24", "32"],
      "DEFAULT": 12
    },
    {
      "NAME": "rayLength",
      "TYPE": "float",
      "DEFAULT": 0.8,
      "MIN": 0.1,
      "MAX": 2.0
    },
    {
      "NAME": "rayWidth",
      "TYPE": "float",
      "DEFAULT": 1.5,
      "MIN": 0.5,
      "MAX": 5.0
    },
    {
      "NAME": "rotationSwitch",
      "TYPE": "bool",
      "DEFAULT": true
    },
    {
      "NAME": "rotationSpeed",
      "TYPE": "float",
      "DEFAULT": 0.3,
      "MIN": 0.0,
      "MAX": 2.0
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
      "NAME": "luminanceThreshold",
      "TYPE": "float",
      "DEFAULT": 0.1,
      "MIN": 0.0,
      "MAX": 1.0
    },
    {
      "NAME": "showImage",
      "TYPE": "float",
      "DEFAULT": 0.3,
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

// Hash function for deterministic pseudo-random values
float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

float hash1(float p) {
  return fract(sin(p * 127.1) * 43758.5453123);
}

float luminance(vec3 c) {
  return dot(c, vec3(0.2126, 0.7152, 0.0722));
}

// Distance from point to line segment
float distToSegment(vec2 p, vec2 a, vec2 b) {
  vec2 ab = b - a;
  vec2 ap = p - a;
  float t = clamp(dot(ap, ab) / dot(ab, ab), 0.0, 1.0);
  return length(ap - ab * t);
}

void main() {
  vec2 uv = isf_FragNormCoord;
  vec2 pixel = uv * RENDERSIZE;
  vec4 original = IMG_NORM_PIXEL(inputImage, uv);

  float cellSize = gridSize;
  vec2 gridPos = floor(pixel / cellSize);

  // Simulated audio amplitude from image average brightness in neighborhood
  float simAmp = luminance(IMG_NORM_PIXEL(inputImage, vec2(0.5)).rgb) * audioReactivity;

  float rayAlpha = 0.0;
  vec3 rayColor = vec3(1.0);

  // Check surrounding grid nodes (3x3 neighborhood)
  for (int dx = -2; dx <= 2; dx++) {
    for (int dy = -2; dy <= 2; dy++) {
      vec2 nodeGrid = gridPos + vec2(float(dx), float(dy));
      vec2 nodePos = nodeGrid * cellSize;
      vec2 nodeUV = clamp(nodePos / RENDERSIZE, 0.0, 1.0);

      // Sample image at node to determine ray properties
      vec3 nodeColor = IMG_NORM_PIXEL(inputImage, nodeUV).rgb;
      float nodeLum = luminance(nodeColor);

      // Skip dim nodes
      if (nodeLum < luminanceThreshold) continue;

      // Number of rays proportional to luminance
      int nRays = int(nodeLum * float(maxRays));
      if (nRays < 1) continue;

      // Rotation direction: deterministic per node, flips with simulated audio
      float dirSeed = hash(nodeGrid * 7.3);
      float rotDir = dirSeed > 0.5 ? 1.0 : -1.0;
      if (simAmp > 0.1 && hash(nodeGrid * 13.7 + floor(TIME * 4.0)) > 0.6) {
        rotDir *= -1.0;
      }

      for (int r = 0; r < 32; r++) {
        if (r >= nRays) break;

        // Base angle from hash
        float angle = hash(nodeGrid * 100.0 + float(r) * 17.31) * 6.2831853;

        // Animated rotation (matches original: rotate by speed * dir / magnitude)
        if (rotationSwitch) {
          float rLen = rayLength * cellSize * (0.5 + hash(nodeGrid + float(r) * 3.1) * 0.5);
          angle += TIME * rotationSpeed * rotDir / max(rLen * 0.01, 0.1);
        }

        // Psycho mode: rays grow with simulated amplitude
        float lengthMult = 1.0;
        if (psychoSwitch) {
          if (hash(nodeGrid * 23.0 + float(r) + floor(TIME * 10.0)) > 0.8) {
            lengthMult = 1.0 + simAmp * 3.0;
          }
        }

        vec2 rayDir = vec2(cos(angle), sin(angle));
        float rLen = rayLength * cellSize * (0.5 + nodeLum * 0.5) * lengthMult;

        // Distance from current pixel to this ray segment
        vec2 toPixel = pixel - nodePos;
        float proj = dot(toPixel, rayDir);

        if (proj > 0.0 && proj < rLen) {
          float dist = length(toPixel - rayDir * proj);
          float alpha = smoothstep(rayWidth + 0.5, rayWidth - 0.5, dist);
          // Fade toward ray tip
          alpha *= 1.0 - (proj / rLen) * 0.3;
          // Original uses white rays with alpha 100/255
          alpha *= 0.39;

          if (psychoSwitch) {
            // Random color per ray in psycho mode
            vec3 pColor = vec3(
              hash(nodeGrid * 3.0 + float(r) + floor(TIME * 8.0)),
              hash(nodeGrid * 5.0 + float(r) + floor(TIME * 8.0) + 1.0),
              hash(nodeGrid * 7.0 + float(r) + floor(TIME * 8.0) + 2.0)
            );
            rayColor = mix(rayColor, pColor, alpha);
          }

          rayAlpha = max(rayAlpha, alpha);
        }
      }
    }
  }

  // Composite: dim original image + ray overlay
  vec3 base = original.rgb * showImage;
  vec3 result;
  if (psychoSwitch) {
    result = base + rayColor * rayAlpha;
  } else {
    result = base + vec3(1.0) * rayAlpha;
  }

  gl_FragColor = vec4(mix(original.rgb, result, intensity), original.a);
}
