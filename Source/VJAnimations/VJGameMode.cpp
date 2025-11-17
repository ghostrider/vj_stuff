// Copyright Epic Games, Inc. All Rights Reserved.

#include "VJGameMode.h"
#include "VJPlayerController.h"

AVJGameMode::AVJGameMode()
{
	// Set default player controller class
	PlayerControllerClass = AVJPlayerController::StaticClass();

	// Disable HUD and pawn for VJ mode
	DefaultPawnClass = nullptr;
	HUDClass = nullptr;
}
