/*{
  "ISFVSN": "2",
  "VSN": "1.0",
  "DESCRIPTION": "GLIC Full Pipeline — Simulates the complete GLIC codec: colorspace conversion, prediction, quantization. Encode with one set of parameters, decode with another to create the signature GLIC glitch aesthetic.",
  "CREDIT": "GLIC port by Claude",
  "CATEGORIES": ["GLIC"],
  "INPUTS": [
    {
      "NAME": "inputImage",
      "TYPE": "image"
    },
    {
      "NAME": "encodeColorspace",
      "TYPE": "long",
      "VALUES": [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15],
      "LABELS": ["OHTA","RGB","CMY","HSB","XYZ","YXY","HCL","LUV","LAB","HWB","RGGBG","YPbPr","YCbCr","YDbDr","Greyscale","YUV"],
      "DEFAULT": 12
    },
    {
      "NAME": "decodeColorspace",
      "TYPE": "long",
      "VALUES": [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15],
      "LABELS": ["OHTA","RGB","CMY","HSB","XYZ","YXY","HCL","LUV","LAB","HWB","RGGBG","YPbPr","YCbCr","YDbDr","Greyscale","YUV"],
      "DEFAULT": 8
    },
    {
      "NAME": "encodePrediction",
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
      "DEFAULT": 1
    },
    {
      "NAME": "quantStep",
      "TYPE": "float",
      "DEFAULT": 16.0,
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
      "NAME": "iterations",
      "TYPE": "long",
      "VALUES": [1,2,3,4,5],
      "LABELS": ["1x","2x","3x","4x","5x"],
      "DEFAULT": 1
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
// Colorspace conversions (compact versions)
// ============================================================

const float PI = 3.14159265359;

float srgbToLinear(float c) {
  return (c <= 0.04045) ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4);
}
float linearToSrgb(float c) {
  return (c <= 0.0031308) ? c * 12.92 : 1.055 * pow(c, 1.0 / 2.4) - 0.055;
}

vec3 rgbToOhta(vec3 c) {
  return vec3((c.r+c.g+c.b)/3.0, (c.r-c.b)/2.0+0.5, (2.0*c.g-c.r-c.b)/4.0+0.5);
}
vec3 ohtaToRgb(vec3 c) {
  float i1=c.x, i2=c.y-0.5, i3=c.z-0.5;
  return vec3(i1+i2-i3, i1+i3, i1-i2-i3);
}

vec3 rgbToCmy(vec3 c) { return vec3(1.0)-c; }
vec3 cmyToRgb(vec3 c) { return vec3(1.0)-c; }

vec3 rgbToHsb(vec3 c) {
  float mx=max(c.r,max(c.g,c.b)), mn=min(c.r,min(c.g,c.b)), d=mx-mn;
  float h=0.0, s=(mx>0.0)?d/mx:0.0;
  if(d>0.001){
    if(mx==c.r){h=(c.g-c.b)/d; if(h<0.0)h+=6.0;}
    else if(mx==c.g) h=2.0+(c.b-c.r)/d;
    else h=4.0+(c.r-c.g)/d;
    h/=6.0;
  }
  return vec3(h,s,mx);
}
vec3 hsbToRgb(vec3 c) {
  float h=c.x*6.0, s=c.y, v=c.z, i=floor(h), f=h-i;
  float p=v*(1.0-s), q=v*(1.0-s*f), t=v*(1.0-s*(1.0-f));
  int hi=int(mod(i,6.0));
  if(hi==0) return vec3(v,t,p); if(hi==1) return vec3(q,v,p);
  if(hi==2) return vec3(p,v,t); if(hi==3) return vec3(p,q,v);
  if(hi==4) return vec3(t,p,v); return vec3(v,p,q);
}

vec3 rgbToXyz(vec3 c) {
  float r=srgbToLinear(c.r), g=srgbToLinear(c.g), b=srgbToLinear(c.b);
  return vec3(0.4124564*r+0.3575761*g+0.1804375*b, 0.2126729*r+0.7151522*g+0.0721750*b, 0.0193339*r+0.1191920*g+0.9503041*b);
}
vec3 xyzToRgb(vec3 c) {
  return vec3(linearToSrgb(3.2404542*c.x-1.5371385*c.y-0.4985314*c.z),
              linearToSrgb(-0.9692660*c.x+1.8760108*c.y+0.0415560*c.z),
              linearToSrgb(0.0556434*c.x-0.2040259*c.y+1.0572252*c.z));
}

vec3 rgbToYxy(vec3 c) {
  vec3 xyz=rgbToXyz(c); float s=xyz.x+xyz.y+xyz.z;
  return (s<0.0001) ? vec3(0.0,0.3127,0.3290) : vec3(xyz.y, xyz.x/s, xyz.y/s);
}
vec3 yxyToRgb(vec3 c) {
  if(c.z<0.0001) return vec3(0.0);
  return xyzToRgb(vec3(c.y*c.x/c.z, c.x, (1.0-c.y-c.z)*c.x/c.z));
}

vec3 rgbToHcl(vec3 c) {
  vec3 xyz=rgbToXyz(c);
  float xr=xyz.x/0.95047, yr=xyz.y, zr=xyz.z/1.08883;
  float fx=(xr>0.008856)?pow(xr,1.0/3.0):7.787*xr+16.0/116.0;
  float fy=(yr>0.008856)?pow(yr,1.0/3.0):7.787*yr+16.0/116.0;
  float fz=(zr>0.008856)?pow(zr,1.0/3.0):7.787*zr+16.0/116.0;
  float L=116.0*fy-16.0, a=500.0*(fx-fy), b=200.0*(fy-fz);
  return vec3(atan(b,a)/(2.0*PI)+0.5, sqrt(a*a+b*b)/150.0, L/100.0);
}
vec3 hclToRgb(vec3 c) {
  float H=(c.x-0.5)*2.0*PI, C=c.y*150.0, L=c.z*100.0;
  float a=C*cos(H), b=C*sin(H);
  float fy=(L+16.0)/116.0, fx=a/500.0+fy, fz=fy-b/200.0;
  float xr=(fx>0.206897)?fx*fx*fx:(fx-16.0/116.0)/7.787;
  float yr=(L>7.9996)?fy*fy*fy:L/903.3;
  float zr=(fz>0.206897)?fz*fz*fz:(fz-16.0/116.0)/7.787;
  return xyzToRgb(vec3(xr*0.95047, yr, zr*1.08883));
}

vec3 rgbToLuv(vec3 c) {
  vec3 xyz=rgbToXyz(c);
  float L=(xyz.y>0.008856)?116.0*pow(xyz.y,1.0/3.0)-16.0:903.3*xyz.y;
  float d=xyz.x+15.0*xyz.y+3.0*xyz.z;
  float up=(d>0.0001)?4.0*xyz.x/d:0.0, vp=(d>0.0001)?9.0*xyz.y/d:0.0;
  return vec3(L/100.0, 13.0*L*(up-0.19783982)/200.0+0.5, 13.0*L*(vp-0.46833630)/200.0+0.5);
}
vec3 luvToRgb(vec3 c) {
  float L=c.x*100.0, u=(c.y-0.5)*200.0, v=(c.z-0.5)*200.0;
  if(L<0.001) return vec3(0.0);
  float up=u/(13.0*L)+0.19783982, vp=v/(13.0*L)+0.46833630;
  float Y=(L>7.9996)?pow((L+16.0)/116.0,3.0):L/903.3;
  return xyzToRgb(vec3((vp>0.0001)?Y*9.0*up/(4.0*vp):0.0, Y, (vp>0.0001)?Y*(12.0-3.0*up-20.0*vp)/(4.0*vp):0.0));
}

vec3 rgbToLab(vec3 c) {
  vec3 xyz=rgbToXyz(c);
  float xr=xyz.x/0.95047, yr=xyz.y, zr=xyz.z/1.08883;
  float fx=(xr>0.008856)?pow(xr,1.0/3.0):7.787*xr+16.0/116.0;
  float fy=(yr>0.008856)?pow(yr,1.0/3.0):7.787*yr+16.0/116.0;
  float fz=(zr>0.008856)?pow(zr,1.0/3.0):7.787*zr+16.0/116.0;
  return vec3((116.0*fy-16.0)/100.0, 500.0*(fx-fy)/256.0+0.5, 200.0*(fy-fz)/256.0+0.5);
}
vec3 labToRgb(vec3 c) {
  float L=c.x*100.0, a=(c.y-0.5)*256.0, b=(c.z-0.5)*256.0;
  float fy=(L+16.0)/116.0, fx=a/500.0+fy, fz=fy-b/200.0;
  float xr=(fx>0.206897)?fx*fx*fx:(fx-16.0/116.0)/7.787;
  float yr=(L>7.9996)?fy*fy*fy:L/903.3;
  float zr=(fz>0.206897)?fz*fz*fz:(fz-16.0/116.0)/7.787;
  return xyzToRgb(vec3(xr*0.95047, yr, zr*1.08883));
}

vec3 rgbToHwb(vec3 c) {
  return vec3(rgbToHsb(c).x, min(c.r,min(c.g,c.b)), 1.0-max(c.r,max(c.g,c.b)));
}
vec3 hwbToRgb(vec3 c) {
  return hsbToRgb(vec3(c.x,1.0,1.0))*(1.0-c.y-c.z)+c.y;
}

vec3 rgbToYpbpr(vec3 c) {
  float y=0.299*c.r+0.587*c.g+0.114*c.b;
  return vec3(y, -0.168736*c.r-0.331264*c.g+0.5*c.b+0.5, 0.5*c.r-0.418688*c.g-0.081312*c.b+0.5);
}
vec3 ypbprToRgb(vec3 c) {
  float y=c.x, pb=c.y-0.5, pr=c.z-0.5;
  return vec3(y+1.402*pr, y-0.344136*pb-0.714136*pr, y+1.772*pb);
}

vec3 rgbToYcbcr(vec3 c) {
  float y=0.299*c.r+0.587*c.g+0.114*c.b;
  return vec3(y, (c.b-y)*0.564+0.5, (c.r-y)*0.713+0.5);
}
vec3 ycbcrToRgb(vec3 c) {
  float y=c.x, cb=c.y-0.5, cr=c.z-0.5;
  return vec3(y+1.402*cr, y-0.344136*cb-0.714136*cr, y+1.772*cb);
}

vec3 rgbToYdbdr(vec3 c) {
  float y=0.299*c.r+0.587*c.g+0.114*c.b;
  return vec3(y, (-0.450*c.r-0.883*c.g+1.333*c.b)/3.0+0.5, (-1.333*c.r+1.116*c.g+0.217*c.b)/3.0+0.5);
}
vec3 ydbdrToRgb(vec3 c) {
  float y=c.x, db=(c.y-0.5)*3.0, dr=(c.z-0.5)*3.0;
  return vec3(y+0.000092*db-0.525912*dr, y-0.129132*db+0.267899*dr, y+0.664679*db-0.000079*dr);
}

vec3 rgbToGs(vec3 c) { return vec3(dot(c, vec3(0.2126,0.7152,0.0722))); }

vec3 rgbToYuv(vec3 c) {
  float y=0.299*c.r+0.587*c.g+0.114*c.b;
  return vec3(y, 0.492*(c.b-y)+0.5, 0.877*(c.r-y)+0.5);
}
vec3 yuvToRgb(vec3 c) {
  float y=c.x, u=c.y-0.5, v=c.z-0.5;
  return vec3(y+1.14*v, y-0.395*u-0.581*v, y+2.033*u);
}

vec3 toCS(int cs, vec3 rgb) {
  if(cs==0) return rgbToOhta(rgb);  if(cs==1) return rgb;
  if(cs==2) return rgbToCmy(rgb);   if(cs==3) return rgbToHsb(rgb);
  if(cs==4) return rgbToXyz(rgb);   if(cs==5) return rgbToYxy(rgb);
  if(cs==6) return rgbToHcl(rgb);   if(cs==7) return rgbToLuv(rgb);
  if(cs==8) return rgbToLab(rgb);   if(cs==9) return rgbToHwb(rgb);
  if(cs==10) return rgb;            if(cs==11) return rgbToYpbpr(rgb);
  if(cs==12) return rgbToYcbcr(rgb);if(cs==13) return rgbToYdbdr(rgb);
  if(cs==14) return rgbToGs(rgb);   if(cs==15) return rgbToYuv(rgb);
  return rgb;
}
vec3 fromCS(int cs, vec3 v) {
  if(cs==0) return ohtaToRgb(v);  if(cs==1) return v;
  if(cs==2) return cmyToRgb(v);   if(cs==3) return hsbToRgb(v);
  if(cs==4) return xyzToRgb(v);   if(cs==5) return yxyToRgb(v);
  if(cs==6) return hclToRgb(v);   if(cs==7) return luvToRgb(v);
  if(cs==8) return labToRgb(v);   if(cs==9) return hwbToRgb(v);
  if(cs==10) return v;             if(cs==11) return ypbprToRgb(v);
  if(cs==12) return ycbcrToRgb(v); if(cs==13) return ydbdrToRgb(v);
  if(cs==14) return v;             if(cs==15) return yuvToRgb(v);
  return v;
}

// ============================================================
// Prediction (neighbor-based)
// ============================================================

vec2 px; // pixel step in normalized coords

vec3 sampleOff(vec2 uv, float dx, float dy) {
  return IMG_NORM_PIXEL(inputImage, clamp(uv + vec2(dx, dy) * px, 0.0, 1.0)).rgb;
}

vec3 paeth(vec3 l, vec3 t, vec3 c) {
  vec3 p = l + t - c;
  vec3 pa = abs(p-l), pb = abs(p-t), pc = abs(p-c);
  vec3 r;
  r.r = (pa.r<=pb.r && pa.r<=pc.r) ? l.r : (pb.r<=pc.r ? t.r : c.r);
  r.g = (pa.g<=pb.g && pa.g<=pc.g) ? l.g : (pb.g<=pc.g ? t.g : c.g);
  r.b = (pa.b<=pb.b && pa.b<=pc.b) ? l.b : (pb.b<=pc.b ? t.b : c.b);
  return r;
}

vec3 predict(int mode, vec2 uv) {
  vec3 l = sampleOff(uv,-1.0,0.0);
  vec3 t = sampleOff(uv,0.0,1.0);
  vec3 c = sampleOff(uv,-1.0,1.0);

  if(mode==0) return c;
  if(mode==1) return l;
  if(mode==2) return t;
  if(mode==3) {
    return (sampleOff(uv,-1.0,0.0)+sampleOff(uv,-2.0,0.0)+sampleOff(uv,0.0,1.0)+sampleOff(uv,0.0,2.0)+c+sampleOff(uv,-2.0,1.0))/6.0;
  }
  if(mode==4) return max(min(l,t), min(max(l,t),c)); // median
  if(mode==5) return l+t-c; // TrueMotion
  if(mode==6) return paeth(l,t,c);
  if(mode==7) { // JPEG-LS
    vec3 r;
    r.r=(c.r>=max(l.r,t.r))?min(l.r,t.r):(c.r<=min(l.r,t.r))?max(l.r,t.r):l.r+t.r-c.r;
    r.g=(c.g>=max(l.g,t.g))?min(l.g,t.g):(c.g<=min(l.g,t.g))?max(l.g,t.g):l.g+t.g-c.g;
    r.b=(c.b>=max(l.b,t.b))?min(l.b,t.b):(c.b<=min(l.b,t.b))?max(l.b,t.b):l.b+t.b-c.b;
    return r;
  }
  if(mode==8) return (l+t)*0.5;
  if(mode==9) return l*0.5+c*0.25+sampleOff(uv,-1.0,-1.0)*0.25;
  if(mode==10) {
    vec2 pp=uv*RENDERSIZE;
    return (mod(pp.x+pp.y,2.0)<1.0)?l:t;
  }
  if(mode==11) return 2.0*l-c;
  if(mode==12) {
    vec3 ri=sampleOff(uv,1.0,0.0), bo=sampleOff(uv,0.0,-1.0);
    vec3 gx=ri-l, gy=t-bo;
    float hs=dot(abs(gy),vec3(1.0)), vs=dot(abs(gx),vec3(1.0)), tot=hs+vs+0.001;
    return l*(hs/tot)+t*(vs/tot);
  }
  return l;
}

// ============================================================
// Quantization
// ============================================================

vec3 quant(vec3 v, float step) {
  return floor(v * 255.0 / step + 0.5) * step / 255.0;
}

// ============================================================
// Full GLIC pipeline: encode → quantize → decode
// ============================================================

vec3 glicPipeline(vec3 rgb, vec2 uv) {
  // --- ENCODE ---
  // 1. Convert to encode colorspace
  vec3 encoded = toCS(encodeColorspace, rgb);

  // 2. Compute prediction in encode colorspace and subtract (get residual)
  vec3 encPredRgb = predict(encodePrediction, uv);
  vec3 encPred = toCS(encodeColorspace, encPredRgb);
  vec3 residual = encoded - encPred;

  // 3. Quantize residual
  vec3 quantized = quant(residual + 0.5, quantStep) - 0.5;

  // 4. Dequantize with scale mismatch
  vec3 dequantized = quantized * dequantScale;

  // --- DECODE (with potentially different parameters) ---
  // 5. Compute decode prediction and add back
  vec3 decPredRgb = predict(decodePrediction, uv);
  vec3 decPred = toCS(decodeColorspace, decPredRgb);
  vec3 reconstructed = decPred + dequantized;

  // 6. Convert from decode colorspace back to RGB
  return fromCS(decodeColorspace, reconstructed);
}

// ============================================================
// Main
// ============================================================

void main() {
  vec2 uv = isf_FragNormCoord;
  px = 1.0 / RENDERSIZE;

  vec4 original = IMG_NORM_PIXEL(inputImage, uv);
  vec3 rgb = original.rgb;

  // Apply pipeline N times (iterations)
  vec3 result = rgb;
  for (int i = 0; i < 5; i++) {
    if (i >= iterations) break;
    result = glicPipeline(result, uv);
    // Clamp between iterations to prevent runaway values
    result = clamp(result, 0.0, 1.0);
  }

  result = mix(rgb, result, intensity);
  gl_FragColor = vec4(result, original.a);
}
