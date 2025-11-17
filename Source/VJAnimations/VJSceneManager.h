// Copyright Epic Games, Inc. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "VJSceneManager.generated.h"

DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FOnSceneChanged, int32, SceneIndex);

/**
 * VJ Scene Manager - Handles switching between different visual scenes
 * Supports up to 10 scenes (0-9) with keyboard shortcuts
 */
UCLASS()
class VJANIMATIONS_API AVJSceneManager : public AActor
{
	GENERATED_BODY()

public:
	AVJSceneManager();

protected:
	virtual void BeginPlay() override;

public:
	virtual void Tick(float DeltaTime) override;

	/** Array of scene actors/levels to switch between */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VJ Scene")
	TArray<AActor*> Scenes;

	/** Current active scene index */
	UPROPERTY(BlueprintReadOnly, Category = "VJ Scene")
	int32 CurrentSceneIndex;

	/** Whether scene transitions are instant or fade */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VJ Scene")
	bool bInstantTransition;

	/** Transition fade duration in seconds */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VJ Scene", meta = (EditCondition = "!bInstantTransition"))
	float TransitionDuration;

	/** Master blackout state */
	UPROPERTY(BlueprintReadOnly, Category = "VJ Scene")
	bool bIsBlackedOut;

	/** Master fade value (0.0 to 1.0) */
	UPROPERTY(BlueprintReadOnly, Category = "VJ Scene")
	float MasterFade;

	/** Event fired when scene changes */
	UPROPERTY(BlueprintAssignable, Category = "VJ Scene")
	FOnSceneChanged OnSceneChanged;

	/** Switch to a specific scene by index */
	UFUNCTION(BlueprintCallable, Category = "VJ Scene")
	void SwitchToScene(int32 SceneIndex);

	/** Switch to next scene */
	UFUNCTION(BlueprintCallable, Category = "VJ Scene")
	void NextScene();

	/** Switch to previous scene */
	UFUNCTION(BlueprintCallable, Category = "VJ Scene")
	void PreviousScene();

	/** Toggle blackout on/off */
	UFUNCTION(BlueprintCallable, Category = "VJ Scene")
	void ToggleBlackout();

	/** Set master fade level */
	UFUNCTION(BlueprintCallable, Category = "VJ Scene")
	void SetMasterFade(float FadeValue);

	/** Fade in master */
	UFUNCTION(BlueprintCallable, Category = "VJ Scene")
	void MasterFadeIn(float DeltaValue = 0.1f);

	/** Fade out master */
	UFUNCTION(BlueprintCallable, Category = "VJ Scene")
	void MasterFadeOut(float DeltaValue = 0.1f);

private:
	void ActivateScene(int32 SceneIndex);
	void DeactivateAllScenes();
};
