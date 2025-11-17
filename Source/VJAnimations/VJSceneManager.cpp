// Copyright Epic Games, Inc. All Rights Reserved.

#include "VJSceneManager.h"
#include "Engine/World.h"

AVJSceneManager::AVJSceneManager()
{
	PrimaryActorTick.bCanEverTick = true;

	CurrentSceneIndex = -1;
	bInstantTransition = true;
	TransitionDuration = 1.0f;
	bIsBlackedOut = false;
	MasterFade = 1.0f;
}

void AVJSceneManager::BeginPlay()
{
	Super::BeginPlay();

	// Activate first scene if available
	if (Scenes.Num() > 0)
	{
		SwitchToScene(0);
	}
}

void AVJSceneManager::Tick(float DeltaTime)
{
	Super::Tick(DeltaTime);
}

void AVJSceneManager::SwitchToScene(int32 SceneIndex)
{
	if (SceneIndex < 0 || SceneIndex >= Scenes.Num())
	{
		UE_LOG(LogTemp, Warning, TEXT("Scene index %d out of range (0-%d)"), SceneIndex, Scenes.Num() - 1);
		return;
	}

	if (SceneIndex == CurrentSceneIndex)
	{
		return; // Already on this scene
	}

	// Deactivate all scenes first
	DeactivateAllScenes();

	// Activate the new scene
	ActivateScene(SceneIndex);

	CurrentSceneIndex = SceneIndex;
	OnSceneChanged.Broadcast(SceneIndex);

	UE_LOG(LogTemp, Log, TEXT("Switched to Scene %d"), SceneIndex);
}

void AVJSceneManager::NextScene()
{
	if (Scenes.Num() == 0) return;

	int32 NextIndex = (CurrentSceneIndex + 1) % Scenes.Num();
	SwitchToScene(NextIndex);
}

void AVJSceneManager::PreviousScene()
{
	if (Scenes.Num() == 0) return;

	int32 PrevIndex = CurrentSceneIndex - 1;
	if (PrevIndex < 0)
	{
		PrevIndex = Scenes.Num() - 1;
	}
	SwitchToScene(PrevIndex);
}

void AVJSceneManager::ToggleBlackout()
{
	bIsBlackedOut = !bIsBlackedOut;
	UE_LOG(LogTemp, Log, TEXT("Blackout: %s"), bIsBlackedOut ? TEXT("ON") : TEXT("OFF"));
}

void AVJSceneManager::SetMasterFade(float FadeValue)
{
	MasterFade = FMath::Clamp(FadeValue, 0.0f, 1.0f);
}

void AVJSceneManager::MasterFadeIn(float DeltaValue)
{
	SetMasterFade(MasterFade + DeltaValue);
	UE_LOG(LogTemp, Log, TEXT("Master Fade: %.2f"), MasterFade);
}

void AVJSceneManager::MasterFadeOut(float DeltaValue)
{
	SetMasterFade(MasterFade - DeltaValue);
	UE_LOG(LogTemp, Log, TEXT("Master Fade: %.2f"), MasterFade);
}

void AVJSceneManager::ActivateScene(int32 SceneIndex)
{
	if (Scenes.IsValidIndex(SceneIndex) && Scenes[SceneIndex])
	{
		Scenes[SceneIndex]->SetActorHiddenInGame(false);
		Scenes[SceneIndex]->SetActorEnableCollision(true);
		Scenes[SceneIndex]->SetActorTickEnabled(true);
	}
}

void AVJSceneManager::DeactivateAllScenes()
{
	for (AActor* Scene : Scenes)
	{
		if (Scene)
		{
			Scene->SetActorHiddenInGame(true);
			Scene->SetActorEnableCollision(false);
			Scene->SetActorTickEnabled(false);
		}
	}
}
