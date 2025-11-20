// Copyright Epic Games, Inc. All Rights Reserved.

#include "VJAudioAnalyzer.h"
#include "AudioCaptureComponent.h"

AVJAudioAnalyzer::AVJAudioAnalyzer()
{
	PrimaryActorTick.bCanEverTick = true;

	// Create audio capture component
	AudioCaptureComponent = CreateDefaultSubobject<UAudioCaptureComponent>(TEXT("AudioCapture"));
	RootComponent = AudioCaptureComponent;

	bAudioReactiveEnabled = true;
	AudioAmplitude = 0.0f;
	LowFrequencyEnergy = 0.0f;
	MidFrequencyEnergy = 0.0f;
	HighFrequencyEnergy = 0.0f;
	AudioIntensity = 0.0f;
	BeatThreshold = 0.6f;
	PreviousAmplitude = 0.0f;
	BeatCooldown = 0.0f;
}

void AVJAudioAnalyzer::BeginPlay()
{
	Super::BeginPlay();

	if (bAudioReactiveEnabled)
	{
		StartAudioCapture();
	}
}

void AVJAudioAnalyzer::Tick(float DeltaTime)
{
	Super::Tick(DeltaTime);

	if (bAudioReactiveEnabled)
	{
		ProcessAudioData();
		DetectBeat();
	}

	// Decay beat cooldown
	if (BeatCooldown > 0.0f)
	{
		BeatCooldown -= DeltaTime;
	}
}

void AVJAudioAnalyzer::ToggleAudioReactive()
{
	bAudioReactiveEnabled = !bAudioReactiveEnabled;

	if (bAudioReactiveEnabled)
	{
		StartAudioCapture();
		UE_LOG(LogTemp, Log, TEXT("Audio Reactive: ON"));
	}
	else
	{
		StopAudioCapture();
		UE_LOG(LogTemp, Log, TEXT("Audio Reactive: OFF"));
	}
}

void AVJAudioAnalyzer::StartAudioCapture()
{
	if (AudioCaptureComponent && !AudioCaptureComponent->IsCapturing())
	{
		// Audio capture will be started automatically by the component
		UE_LOG(LogTemp, Log, TEXT("Audio capture started"));
	}
}

void AVJAudioAnalyzer::StopAudioCapture()
{
	if (AudioCaptureComponent && AudioCaptureComponent->IsCapturing())
	{
		// Stop audio capture
		UE_LOG(LogTemp, Log, TEXT("Audio capture stopped"));
	}
}

float AVJAudioAnalyzer::GetFrequencyValue(float MinFrequency, float MaxFrequency)
{
	// This is a simplified implementation
	// In a full implementation, you would use AudioSynesthesia or FFT analysis
	if (MinFrequency < 250.0f)
	{
		return LowFrequencyEnergy;
	}
	else if (MinFrequency < 2000.0f)
	{
		return MidFrequencyEnergy;
	}
	else
	{
		return HighFrequencyEnergy;
	}
}

void AVJAudioAnalyzer::ProcessAudioData()
{
	// This is a placeholder for audio processing
	// In a full implementation, you would:
	// 1. Get audio buffer from AudioCaptureComponent
	// 2. Perform FFT or use AudioSynesthesia
	// 3. Calculate frequency band energies
	// 4. Update amplitude and intensity values

	// For now, we'll use placeholder values that can be set from Blueprints
	// or by integrating with Unreal's audio analysis systems

	// Calculate overall intensity
	AudioIntensity = (LowFrequencyEnergy * 0.4f + MidFrequencyEnergy * 0.3f + HighFrequencyEnergy * 0.3f);
	AudioIntensity = FMath::Clamp(AudioIntensity, 0.0f, 1.0f);
}

void AVJAudioAnalyzer::DetectBeat()
{
	// Simple beat detection based on amplitude change
	float AmplitudeDelta = AudioAmplitude - PreviousAmplitude;

	if (AmplitudeDelta > BeatThreshold && BeatCooldown <= 0.0f)
	{
		OnBeatDetected.Broadcast(AmplitudeDelta);
		BeatCooldown = 0.2f; // Minimum time between beats
		UE_LOG(LogTemp, Verbose, TEXT("Beat detected! Strength: %.2f"), AmplitudeDelta);
	}

	PreviousAmplitude = AudioAmplitude;
}
