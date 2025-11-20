// Copyright Epic Games, Inc. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "AudioCaptureComponent.h"
#include "Sound/SoundSubmix.h"
#include "VJAudioAnalyzer.generated.h"

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnBeatDetected, float, BeatStrength);

/**
 * VJ Audio Analyzer - Captures and analyzes audio for reactive visuals
 * Provides spectrum analysis, beat detection, and audio envelope data
 */
UCLASS()
class VJANIMATIONS_API AVJAudioAnalyzer : public AActor
{
	GENERATED_BODY()

public:
	AVJAudioAnalyzer();

protected:
	virtual void BeginPlay() override;

public:
	virtual void Tick(float DeltaTime) override;

	/** Audio Capture Component */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "VJ Audio")
	UAudioCaptureComponent* AudioCaptureComponent;

	/** Enable audio reactive mode */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VJ Audio")
	bool bAudioReactiveEnabled;

	/** Audio amplitude (volume level) */
	UPROPERTY(BlueprintReadOnly, Category = "VJ Audio")
	float AudioAmplitude;

	/** Low frequency band energy (bass) */
	UPROPERTY(BlueprintReadOnly, Category = "VJ Audio")
	float LowFrequencyEnergy;

	/** Mid frequency band energy */
	UPROPERTY(BlueprintReadOnly, Category = "VJ Audio")
	float MidFrequencyEnergy;

	/** High frequency band energy (treble) */
	UPROPERTY(BlueprintReadOnly, Category = "VJ Audio")
	float HighFrequencyEnergy;

	/** Overall audio intensity (0.0 to 1.0) */
	UPROPERTY(BlueprintReadOnly, Category = "VJ Audio")
	float AudioIntensity;

	/** Beat detection threshold */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VJ Audio")
	float BeatThreshold;

	/** Event fired when beat is detected */
	UPROPERTY(BlueprintAssignable, Category = "VJ Audio")
	FOnBeatDetected OnBeatDetected;

	/** Toggle audio reactive mode */
	UFUNCTION(BlueprintCallable, Category = "VJ Audio")
	void ToggleAudioReactive();

	/** Start capturing audio */
	UFUNCTION(BlueprintCallable, Category = "VJ Audio")
	void StartAudioCapture();

	/** Stop capturing audio */
	UFUNCTION(BlueprintCallable, Category = "VJ Audio")
	void StopAudioCapture();

	/** Get normalized audio value for a specific frequency range */
	UFUNCTION(BlueprintCallable, Category = "VJ Audio")
	float GetFrequencyValue(float MinFrequency, float MaxFrequency);

private:
	void ProcessAudioData();
	void DetectBeat();

	float PreviousAmplitude;
	float BeatCooldown;
};
