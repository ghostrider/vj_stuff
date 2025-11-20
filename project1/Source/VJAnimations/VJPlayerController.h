// Copyright Epic Games, Inc. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "GameFramework/PlayerController.h"
#include "VJPlayerController.generated.h"

class AVJSceneManager;
class AVJAudioAnalyzer;

/**
 * VJ Player Controller - Handles input for VJ scene control
 */
UCLASS()
class VJANIMATIONS_API AVJPlayerController : public APlayerController
{
	GENERATED_BODY()

public:
	AVJPlayerController();

protected:
	virtual void BeginPlay() override;
	virtual void SetupInputComponent() override;

public:
	/** Reference to the scene manager */
	UPROPERTY(BlueprintReadWrite, Category = "VJ Control")
	AVJSceneManager* SceneManager;

	/** Reference to the audio analyzer */
	UPROPERTY(BlueprintReadWrite, Category = "VJ Control")
	AVJAudioAnalyzer* AudioAnalyzer;

protected:
	// Scene switching input handlers
	void OnSwitchToScene1();
	void OnSwitchToScene2();
	void OnSwitchToScene3();
	void OnSwitchToScene4();
	void OnSwitchToScene5();
	void OnSwitchToScene6();
	void OnSwitchToScene7();
	void OnSwitchToScene8();
	void OnSwitchToScene9();
	void OnSwitchToScene0();

	// Control input handlers
	void OnToggleAudioReactive();
	void OnNextScene();
	void OnPreviousScene();
	void OnBlackoutToggle();
	void OnMasterFadeIn();
	void OnMasterFadeOut();

	// Audio intensity axis
	void OnAudioIntensity(float Value);

private:
	void FindSceneManager();
	void FindAudioAnalyzer();
};
