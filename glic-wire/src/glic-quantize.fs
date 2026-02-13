/*{
  "ISFVSN": "2",
  "VSN": "1.0",
  "DESCRIPTION": "GLIC Quantization Glitch — Reduce color precision in a selectable colorspace. Mismatch between quantize and dequantize steps creates banding and posterization glitches.",
  "CREDIT": "GLIC port by Claude",
  "CATEGORIES": ["GLIC"],
  "INPUTS": [
    {
      "NAME": "inputImage",
      "TYPE": "image"
    },
    {
      "NAME": "colorspace",
      "TYPE": "long",
      "VALUES": [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15],
      "LABELS": ["OHTA","RGB","CMY","HSB","XYZ","YXY","HCL","LUV","LAB","HWB","RGGBG","YPbPr","YCbCr","YDbDr","Greyscale","YUV"],
      "DEFAULT": 1
    },
    {
      "NAME": "quantStepA",
      "TYPE": "float",
      "DEFAULT": 8.0,
      "MIN": 1.0,
      "MAX": 128.0
    },
    {
      "NAME": "quantStepB",
      "TYPE": "float",
      "DEFAULT": 8.0,
      "MIN": 1.0,
      "MAX": 128.0
    },
    {
      "NAME": "quantStepC",
      "TYPE": "float",
      "DEFAULT": 8.0,
      "MIN": 1.0,
      "MAX": 128.0
    },
    {
      "NAME": "dequantScale",
      "TYPE": "float",
      "DEFAULT": 1.0,
      "MIN": 0.1,
      "MAX": 4.0
    },
    {
      "NAME": "dither",
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

// ============================================================
// Colorspace conversion functions (shared with glic-colorspace)
// ISF has no #include, so these are duplicated here.
// ============================================================

const float PI = 3.14159265359;

float srgbToLinear(float c) {
  return (c <= 0.04045) ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4);
}
float linearToSrgb(float c) {
  return (c <= 0.0031308) ? c * 12.92 : 1.055 * pow(c, 1.0 / 2.4) - 0.055;
}

// 0: OHTA
vec3 rgbToOhta(vec3 c) {
  return vec3(
    (c.r + c.g + c.b) / 3.0,
    (c.r - c.b) / 2.0 + 0.5,
    (2.0 * c.g - c.r - c.b) / 4.0 + 0.5
  );
}
vec3 ohtaToRgb(vec3 c) {
  float i1 = c.x, i2 = c.y - 0.5, i3 = c.z - 0.5;
  return vec3(i1 + i2 - i3, i1 + i3, i1 - i2 - i3);
}

// 1: RGB
vec3 rgbToRgb(vec3 c) { return c; }
vec3 rgbFromRgb(vec3 c) { return c; }

// 2: CMY
vec3 rgbToCmy(vec3 c) { return vec3(1.0) - c; }
vec3 cmyToRgb(vec3 c) { return vec3(1.0) - c; }

// 3: HSB
vec3 rgbToHsb(vec3 c) {
  float maxC = max(c.r, max(c.g, c.b));
  float minC = min(c.r, min(c.g, c.b));
  float delta = maxC - minC;
  float h = 0.0, s = (maxC > 0.0) ? delta / maxC : 0.0, v = maxC;
  if (delta > 0.001) {
    if (maxC == c.r) { h = (c.g - c.b) / delta; if (h < 0.0) h += 6.0; }
    else if (maxC == c.g) { h = 2.0 + (c.b - c.r) / delta; }
    else { h = 4.0 + (c.r - c.g) / delta; }
    h /= 6.0;
  }
  return vec3(h, s, v);
}
vec3 hsbToRgb(vec3 c) {
  float h = c.x * 6.0, s = c.y, v = c.z;
  float i = floor(h), f = h - i;
  float p = v * (1.0 - s), q = v * (1.0 - s * f), t = v * (1.0 - s * (1.0 - f));
  int hi = int(mod(i, 6.0));
  if (hi == 0) return vec3(v, t, p);
  if (hi == 1) return vec3(q, v, p);
  if (hi == 2) return vec3(p, v, t);
  if (hi == 3) return vec3(p, q, v);
  if (hi == 4) return vec3(t, p, v);
  return vec3(v, p, q);
}

// 4: XYZ
vec3 rgbToXyz(vec3 c) {
  float r = srgbToLinear(c.r), g = srgbToLinear(c.g), b = srgbToLinear(c.b);
  return vec3(
    0.4124564*r + 0.3575761*g + 0.1804375*b,
    0.2126729*r + 0.7151522*g + 0.0721750*b,
    0.0193339*r + 0.1191920*g + 0.9503041*b
  );
}
vec3 xyzToRgb(vec3 c) {
  float r =  3.2404542*c.x - 1.5371385*c.y - 0.4985314*c.z;
  float g = -0.9692660*c.x + 1.8760108*c.y + 0.0415560*c.z;
  float b =  0.0556434*c.x - 0.2040259*c.y + 1.0572252*c.z;
  return vec3(linearToSrgb(r), linearToSrgb(g), linearToSrgb(b));
}

// 5: YXY
vec3 rgbToYxy(vec3 c) {
  vec3 xyz = rgbToXyz(c);
  float sum = xyz.x + xyz.y + xyz.z;
  if (sum < 0.0001) return vec3(0.0, 0.3127, 0.3290);
  return vec3(xyz.y, xyz.x / sum, xyz.y / sum);
}
vec3 yxyToRgb(vec3 c) {
  if (c.z < 0.0001) return vec3(0.0);
  float X = c.y * c.x / c.z;
  float Z = (1.0 - c.y - c.z) * c.x / c.z;
  return xyzToRgb(vec3(X, c.x, Z));
}

// 6: HCL
vec3 rgbToHcl(vec3 c) {
  vec3 xyz = rgbToXyz(c);
  float xr = xyz.x/0.95047, yr = xyz.y, zr = xyz.z/1.08883;
  float fx = (xr > 0.008856) ? pow(xr, 1.0/3.0) : 7.787*xr + 16.0/116.0;
  float fy = (yr > 0.008856) ? pow(yr, 1.0/3.0) : 7.787*yr + 16.0/116.0;
  float fz = (zr > 0.008856) ? pow(zr, 1.0/3.0) : 7.787*zr + 16.0/116.0;
  float L = 116.0*fy - 16.0, a = 500.0*(fx - fy), b = 200.0*(fy - fz);
  return vec3(atan(b, a)/(2.0*PI) + 0.5, sqrt(a*a + b*b)/150.0, L/100.0);
}
vec3 hclToRgb(vec3 c) {
  float H = (c.x - 0.5)*2.0*PI, C = c.y*150.0, L = c.z*100.0;
  float a = C*cos(H), b = C*sin(H);
  float fy = (L + 16.0)/116.0, fx = a/500.0 + fy, fz = fy - b/200.0;
  float xr = (fx > 0.206897) ? fx*fx*fx : (fx - 16.0/116.0)/7.787;
  float yr = (L > 7.9996) ? fy*fy*fy : L/903.3;
  float zr = (fz > 0.206897) ? fz*fz*fz : (fz - 16.0/116.0)/7.787;
  return xyzToRgb(vec3(xr*0.95047, yr, zr*1.08883));
}

// 7: LUV
vec3 rgbToLuv(vec3 c) {
  vec3 xyz = rgbToXyz(c);
  float yr = xyz.y;
  float L = (yr > 0.008856) ? 116.0*pow(yr, 1.0/3.0) - 16.0 : 903.3*yr;
  float d = xyz.x + 15.0*xyz.y + 3.0*xyz.z;
  float up = (d > 0.0001) ? 4.0*xyz.x/d : 0.0;
  float vp = (d > 0.0001) ? 9.0*xyz.y/d : 0.0;
  float u = 13.0*L*(up - 0.19783982);
  float v = 13.0*L*(vp - 0.46833630);
  return vec3(L/100.0, u/200.0 + 0.5, v/200.0 + 0.5);
}
vec3 luvToRgb(vec3 c) {
  float L = c.x*100.0, u = (c.y - 0.5)*200.0, v = (c.z - 0.5)*200.0;
  if (L < 0.001) return vec3(0.0);
  float up = u/(13.0*L) + 0.19783982;
  float vp = v/(13.0*L) + 0.46833630;
  float Y = (L > 7.9996) ? pow((L + 16.0)/116.0, 3.0) : L/903.3;
  float X = (vp > 0.0001) ? Y*9.0*up/(4.0*vp) : 0.0;
  float Z = (vp > 0.0001) ? Y*(12.0 - 3.0*up - 20.0*vp)/(4.0*vp) : 0.0;
  return xyzToRgb(vec3(X, Y, Z));
}

// 8: LAB
vec3 rgbToLab(vec3 c) {
  vec3 xyz = rgbToXyz(c);
  float xr = xyz.x/0.95047, yr = xyz.y, zr = xyz.z/1.08883;
  float fx = (xr > 0.008856) ? pow(xr, 1.0/3.0) : 7.787*xr + 16.0/116.0;
  float fy = (yr > 0.008856) ? pow(yr, 1.0/3.0) : 7.787*yr + 16.0/116.0;
  float fz = (zr > 0.008856) ? pow(zr, 1.0/3.0) : 7.787*zr + 16.0/116.0;
  return vec3((116.0*fy - 16.0)/100.0, 500.0*(fx - fy)/256.0 + 0.5, 200.0*(fy - fz)/256.0 + 0.5);
}
vec3 labToRgb(vec3 c) {
  float L = c.x*100.0, a = (c.y - 0.5)*256.0, b = (c.z - 0.5)*256.0;
  float fy = (L + 16.0)/116.0, fx = a/500.0 + fy, fz = fy - b/200.0;
  float xr = (fx > 0.206897) ? fx*fx*fx : (fx - 16.0/116.0)/7.787;
  float yr = (L > 7.9996) ? fy*fy*fy : L/903.3;
  float zr = (fz > 0.206897) ? fz*fz*fz : (fz - 16.0/116.0)/7.787;
  return xyzToRgb(vec3(xr*0.95047, yr, zr*1.08883));
}

// 9: HWB
vec3 rgbToHwb(vec3 c) {
  return vec3(rgbToHsb(c).x, min(c.r, min(c.g, c.b)), 1.0 - max(c.r, max(c.g, c.b)));
}
vec3 hwbToRgb(vec3 c) {
  return hsbToRgb(vec3(c.x, 1.0, 1.0)) * (1.0 - c.y - c.z) + c.y;
}

// 10: RGGBG
vec3 rgbToRggbg(vec3 c) { return vec3(c.r, c.g, c.b); }
vec3 rggbgToRgb(vec3 c) { return c; }

// 11: YPbPr
vec3 rgbToYpbpr(vec3 c) {
  float y = 0.299*c.r + 0.587*c.g + 0.114*c.b;
  return vec3(y, -0.168736*c.r - 0.331264*c.g + 0.5*c.b + 0.5, 0.5*c.r - 0.418688*c.g - 0.081312*c.b + 0.5);
}
vec3 ypbprToRgb(vec3 c) {
  float y = c.x, pb = c.y - 0.5, pr = c.z - 0.5;
  return vec3(y + 1.402*pr, y - 0.344136*pb - 0.714136*pr, y + 1.772*pb);
}

// 12: YCbCr
vec3 rgbToYcbcr(vec3 c) {
  float y = 0.299*c.r + 0.587*c.g + 0.114*c.b;
  return vec3(y, (c.b - y)*0.564 + 0.5, (c.r - y)*0.713 + 0.5);
}
vec3 ycbcrToRgb(vec3 c) {
  float y = c.x, cb = c.y - 0.5, cr = c.z - 0.5;
  return vec3(y + 1.402*cr, y - 0.344136*cb - 0.714136*cr, y + 1.772*cb);
}

// 13: YDbDr
vec3 rgbToYdbdr(vec3 c) {
  float y = 0.299*c.r + 0.587*c.g + 0.114*c.b;
  return vec3(y, (-0.450*c.r - 0.883*c.g + 1.333*c.b)/3.0 + 0.5, (-1.333*c.r + 1.116*c.g + 0.217*c.b)/3.0 + 0.5);
}
vec3 ydbdrToRgb(vec3 c) {
  float y = c.x, db = (c.y - 0.5)*3.0, dr = (c.z - 0.5)*3.0;
  return vec3(y + 0.000092*db - 0.525912*dr, y - 0.129132*db + 0.267899*dr, y + 0.664679*db - 0.000079*dr);
}

// 14: Greyscale
vec3 rgbToGs(vec3 c) { float g = dot(c, vec3(0.2126, 0.7152, 0.0722)); return vec3(g); }
vec3 gsToRgb(vec3 c) { return c; }

// 15: YUV
vec3 rgbToYuv(vec3 c) {
  float y = 0.299*c.r + 0.587*c.g + 0.114*c.b;
  return vec3(y, 0.492*(c.b - y) + 0.5, 0.877*(c.r - y) + 0.5);
}
vec3 yuvToRgb(vec3 c) {
  float y = c.x, u = c.y - 0.5, v = c.z - 0.5;
  return vec3(y + 1.14*v, y - 0.395*u - 0.581*v, y + 2.033*u);
}

// Dispatch
vec3 toColorspace(int cs, vec3 rgb) {
  if (cs == 0) return rgbToOhta(rgb);
  if (cs == 1) return rgb;
  if (cs == 2) return rgbToCmy(rgb);
  if (cs == 3) return rgbToHsb(rgb);
  if (cs == 4) return rgbToXyz(rgb);
  if (cs == 5) return rgbToYxy(rgb);
  if (cs == 6) return rgbToHcl(rgb);
  if (cs == 7) return rgbToLuv(rgb);
  if (cs == 8) return rgbToLab(rgb);
  if (cs == 9) return rgbToHwb(rgb);
  if (cs == 10) return rgbToRggbg(rgb);
  if (cs == 11) return rgbToYpbpr(rgb);
  if (cs == 12) return rgbToYcbcr(rgb);
  if (cs == 13) return rgbToYdbdr(rgb);
  if (cs == 14) return rgbToGs(rgb);
  if (cs == 15) return rgbToYuv(rgb);
  return rgb;
}
vec3 fromColorspace(int cs, vec3 val) {
  if (cs == 0) return ohtaToRgb(val);
  if (cs == 1) return val;
  if (cs == 2) return cmyToRgb(val);
  if (cs == 3) return hsbToRgb(val);
  if (cs == 4) return xyzToRgb(val);
  if (cs == 5) return yxyToRgb(val);
  if (cs == 6) return hclToRgb(val);
  if (cs == 7) return luvToRgb(val);
  if (cs == 8) return labToRgb(val);
  if (cs == 9) return hwbToRgb(val);
  if (cs == 10) return rggbgToRgb(val);
  if (cs == 11) return ypbprToRgb(val);
  if (cs == 12) return ycbcrToRgb(val);
  if (cs == 13) return ydbdrToRgb(val);
  if (cs == 14) return gsToRgb(val);
  if (cs == 15) return yuvToRgb(val);
  return val;
}

// ============================================================
// Pseudo-random hash for dithering
// ============================================================

float hash(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}

// ============================================================
// Main
// ============================================================

void main() {
  vec2 uv = isf_FragNormCoord;
  vec4 original = IMG_NORM_PIXEL(inputImage, uv);
  vec3 rgb = original.rgb;

  // Convert to working colorspace
  vec3 encoded = toColorspace(colorspace, rgb);

  // Add dither noise before quantization
  if (dither > 0.001) {
    vec2 pixelPos = uv * RENDERSIZE;
    vec3 noise = vec3(
      hash(pixelPos + 0.0),
      hash(pixelPos + 137.0),
      hash(pixelPos + 274.0)
    );
    // Map noise to [-0.5, 0.5] range, scale by dither amount and quant step
    noise = (noise - 0.5) * dither;
    encoded += noise * vec3(quantStepA, quantStepB, quantStepC) / 255.0;
  }

  // Quantize each channel independently
  vec3 steps = vec3(quantStepA, quantStepB, quantStepC);
  vec3 quantized = floor(encoded * 255.0 / steps + 0.5) * steps / 255.0;

  // Dequantize with mismatch scale (scale != 1.0 = glitch)
  vec3 dequantized = quantized * dequantScale;

  // Convert back to RGB
  vec3 decoded = fromColorspace(colorspace, dequantized);

  // Mix with original
  vec3 result = mix(rgb, decoded, intensity);

  gl_FragColor = vec4(result, original.a);
}
