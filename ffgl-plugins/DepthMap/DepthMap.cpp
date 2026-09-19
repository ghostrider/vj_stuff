#include "DepthMap.h"
using namespace ffglex;

enum ParamType : FFUInt32
{
	PT_MIX
};

static CFFGLPluginInfo PluginInfo(
	PluginFactory< DepthMap >,                                  // Create method
	"VJDM",                                                     // Plugin unique ID of maximum length 4.
	"DepthMap",                                                 // Plugin name
	2,                                                          // API major version number
	1,                                                          // API minor version number
	1,                                                          // Plugin major version number
	0,                                                          // Plugin minor version number
	FF_EFFECT,                                                  // Plugin type
	"Blends between the input and a greyscale version of it",   // Plugin description
	"vj_stuff DepthMap (toolchain stub, depth inference to follow)"// About
);

static const char _vertexShaderCode[] = R"(#version 410 core
uniform vec2 MaxUV;

layout( location = 0 ) in vec4 vPosition;
layout( location = 1 ) in vec2 vUV;

out vec2 uv;

void main()
{
	gl_Position = vPosition;
	uv = vUV * MaxUV;
}
)";

static const char _fragmentShaderCode[] = R"(#version 410 core
uniform sampler2D InputTexture;
uniform float Mix;

in vec2 uv;

out vec4 fragColor;

void main()
{
	vec4 color = texture( InputTexture, uv );
	//The InputTexture contains premultiplied colors, so we need to unpremultiply first to apply our effect on straight colors.
	if( color.a > 0.0 )
		color.rgb /= color.a;

	float grey = dot( color.rgb, vec3( 0.2126, 0.7152, 0.0722 ) );
	color.rgb  = mix( color.rgb, vec3( grey ), Mix );

	//The plugin has to output premultiplied colors, this is how we're premultiplying our straight color while also
	//ensuring we aren't going out of the LDR the video engine is working in.
	color.rgb = clamp( color.rgb * color.a, vec3( 0.0 ), vec3( color.a ) );
	fragColor = color;
}
)";

DepthMap::DepthMap() :
	mix( 1.0f )
{
	SetMinInputs( 1 );
	SetMaxInputs( 1 );

	//Must come after mix is initialised: the default value reported to the host is read back through GetFloatParameter.
	SetParamInfof( PT_MIX, "Mix", FF_TYPE_STANDARD );

	FFGLLog::LogToHost( "Created DepthMap effect" );
}
DepthMap::~DepthMap()
{
}

FFResult DepthMap::InitGL( const FFGLViewportStruct* vp )
{
	if( !shader.Compile( _vertexShaderCode, _fragmentShaderCode ) )
	{
		DeInitGL();
		return FF_FAIL;
	}
	if( !quad.Initialise() )
	{
		DeInitGL();
		return FF_FAIL;
	}

	//Use base-class init as success result so that it retains the viewport.
	return CFFGLPlugin::InitGL( vp );
}
FFResult DepthMap::ProcessOpenGL( ProcessOpenGLStruct* pGL )
{
	if( pGL->numInputTextures < 1 )
		return FF_FAIL;

	if( pGL->inputTextures[ 0 ] == NULL )
		return FF_FAIL;

	//FFGL requires us to leave the context in a default state on return, so use this scoped binding to help us do that.
	ScopedShaderBinding shaderBinding( shader.GetGLID() );
	//The shader's sampler is always bound to sampler index 0 so that's where we need to bind the texture.
	//Again, we're using the scoped bindings to help us keep the context in a default state.
	ScopedSamplerActivation activateSampler( 0 );
	Scoped2DTextureBinding textureBinding( pGL->inputTextures[ 0 ]->Handle );

	shader.Set( "InputTexture", 0 );

	//The input texture's dimension might change each frame and so might the content area.
	//We're adopting the texture's maxUV using a uniform because that way we dont have to update our vertex buffer each frame.
	FFGLTexCoords maxCoords = GetMaxGLTexCoords( *pGL->inputTextures[ 0 ] );
	shader.Set( "MaxUV", maxCoords.s, maxCoords.t );

	shader.Set( "Mix", mix );

	quad.Draw();

	return FF_SUCCESS;
}
FFResult DepthMap::DeInitGL()
{
	shader.FreeGLResources();
	quad.Release();

	return FF_SUCCESS;
}

FFResult DepthMap::SetFloatParameter( unsigned int dwIndex, float value )
{
	switch( dwIndex )
	{
	case PT_MIX:
		mix = value;
		break;

	default:
		return FF_FAIL;
	}

	return FF_SUCCESS;
}

float DepthMap::GetFloatParameter( unsigned int index )
{
	switch( index )
	{
	case PT_MIX:
		return mix;
	}

	return 0.0f;
}
