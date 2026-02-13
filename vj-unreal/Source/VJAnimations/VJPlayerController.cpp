// Copyright Epic Games, Inc. All Rights Reserved.

#include "VJPlayerController.h"
#include "VJSceneManager.h"
#include "VJAudioAnalyzer.h"
#include "Kismet/GameplayStatics.h"

AVJPlayerController::AVJPlayerController()
{
	bShowMouseCursor = false;
	bEnableClickEvents = false;
	bEnableMouseOverEvents = false;
}

void AVJPlayerController::BeginPlay()
{
	Super::BeginPlay();

	// Find scene manager and audio analyzer in the level
	FindSceneManager();
	FindAudioAnalyzer();
}

void AVJPlayerController::SetupInputComponent()
{
	Super::SetupInputComponent();

	if (InputComponent)
	{
		// Scene switching
		InputComponent->BindAction("SwitchToScene1", IE_Pressed, this, &AVJPlayerController::OnSwitchToScene1);
		InputComponent->BindAction("SwitchToScene2", IE_Pressed, this, &AVJPlayerController::OnSwitchToScene2);
		InputComponent->BindAction("SwitchToScene3", IE_Pressed, this, &AVJPlayerController::OnSwitchToScene3);
		InputComponent->BindAction("SwitchToScene4", IE_Pressed, this, &AVJPlayerController::OnSwitchToScene4);
		InputComponent->BindAction("SwitchToScene5", IE_Pressed, this, &AVJPlayerController::OnSwitchToScene5);
		InputComponent->BindAction("SwitchToScene6", IE_Pressed, this, &AVJPlayerController::OnSwitchToScene6);
		InputComponent->BindAction("SwitchToScene7", IE_Pressed, this, &AVJPlayerController::OnSwitchToScene7);
		InputComponent->BindAction("SwitchToScene8", IE_Pressed, this, &AVJPlayerController::OnSwitchToScene8);
		InputComponent->BindAction("SwitchToScene9", IE_Pressed, this, &AVJPlayerController::OnSwitchToScene9);
		InputComponent->BindAction("SwitchToScene0", IE_Pressed, this, &AVJPlayerController::OnSwitchToScene0);

		// Control actions
		InputComponent->BindAction("ToggleAudioReactive", IE_Pressed, this, &AVJPlayerController::OnToggleAudioReactive);
		InputComponent->BindAction("NextScene", IE_Pressed, this, &AVJPlayerController::OnNextScene);
		InputComponent->BindAction("PreviousScene", IE_Pressed, this, &AVJPlayerController::OnPreviousScene);
		InputComponent->BindAction("BlackoutToggle", IE_Pressed, this, &AVJPlayerController::OnBlackoutToggle);
		InputComponent->BindAction("MasterFadeIn", IE_Pressed, this, &AVJPlayerController::OnMasterFadeIn);
		InputComponent->BindAction("MasterFadeOut", IE_Pressed, this, &AVJPlayerController::OnMasterFadeOut);

		// Axis mappings
		InputComponent->BindAxis("AudioIntensity", this, &AVJPlayerController::OnAudioIntensity);
	}
}

void AVJPlayerController::OnSwitchToScene1() { if (SceneManager) SceneManager->SwitchToScene(0); }
void AVJPlayerController::OnSwitchToScene2() { if (SceneManager) SceneManager->SwitchToScene(1); }
void AVJPlayerController::OnSwitchToScene3() { if (SceneManager) SceneManager->SwitchToScene(2); }
void AVJPlayerController::OnSwitchToScene4() { if (SceneManager) SceneManager->SwitchToScene(3); }
void AVJPlayerController::OnSwitchToScene5() { if (SceneManager) SceneManager->SwitchToScene(4); }
void AVJPlayerController::OnSwitchToScene6() { if (SceneManager) SceneManager->SwitchToScene(5); }
void AVJPlayerController::OnSwitchToScene7() { if (SceneManager) SceneManager->SwitchToScene(6); }
void AVJPlayerController::OnSwitchToScene8() { if (SceneManager) SceneManager->SwitchToScene(7); }
void AVJPlayerController::OnSwitchToScene9() { if (SceneManager) SceneManager->SwitchToScene(8); }
void AVJPlayerController::OnSwitchToScene0() { if (SceneManager) SceneManager->SwitchToScene(9); }

void AVJPlayerController::OnToggleAudioReactive()
{
	if (AudioAnalyzer)
	{
		AudioAnalyzer->ToggleAudioReactive();
	}
}

void AVJPlayerController::OnNextScene()
{
	if (SceneManager)
	{
		SceneManager->NextScene();
	}
}

void AVJPlayerController::OnPreviousScene()
{
	if (SceneManager)
	{
		SceneManager->PreviousScene();
	}
}

void AVJPlayerController::OnBlackoutToggle()
{
	if (SceneManager)
	{
		SceneManager->ToggleBlackout();
	}
}

void AVJPlayerController::OnMasterFadeIn()
{
	if (SceneManager)
	{
		SceneManager->MasterFadeIn();
	}
}

void AVJPlayerController::OnMasterFadeOut()
{
	if (SceneManager)
	{
		SceneManager->MasterFadeOut();
	}
}

void AVJPlayerController::OnAudioIntensity(float Value)
{
	// Mouse wheel control for audio intensity or other parameters
	// Can be used to adjust sensitivity or other real-time parameters
}

void AVJPlayerController::FindSceneManager()
{
	TArray<AActor*> FoundActors;
	UGameplayStatics::GetAllActorsOfClass(GetWorld(), AVJSceneManager::StaticClass(), FoundActors);

	if (FoundActors.Num() > 0)
	{
		SceneManager = Cast<AVJSceneManager>(FoundActors[0]);
		UE_LOG(LogTemp, Log, TEXT("Scene Manager found"));
	}
	else
	{
		UE_LOG(LogTemp, Warning, TEXT("No Scene Manager found in level"));
	}
}

void AVJPlayerController::FindAudioAnalyzer()
{
	TArray<AActor*> FoundActors;
	UGameplayStatics::GetAllActorsOfClass(GetWorld(), AVJAudioAnalyzer::StaticClass(), FoundActors);

	if (FoundActors.Num() > 0)
	{
		AudioAnalyzer = Cast<AVJAudioAnalyzer>(FoundActors[0]);
		UE_LOG(LogTemp, Log, TEXT("Audio Analyzer found"));
	}
	else
	{
		UE_LOG(LogTemp, Warning, TEXT("No Audio Analyzer found in level"));
	}
}
