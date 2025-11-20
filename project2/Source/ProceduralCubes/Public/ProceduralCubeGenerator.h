// Copyright Epic Games, Inc. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "ProceduralCubeGenerator.generated.h"

UCLASS()
class PROCEDURALCUBES_API AProceduralCubeGenerator : public AActor
{
	GENERATED_BODY()

public:
	// Sets default values for this actor's properties
	AProceduralCubeGenerator();

	// Called when the actor is constructed or placed in the world
	virtual void OnConstruction(const FTransform& Transform) override;

protected:
	// Called when the game starts or when spawned
	virtual void BeginPlay() override;

public:
	// Cube dimensions (number of boxes in each direction)
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Procedural Cube")
	int32 CubeSize = 10;

	// Size of each individual box
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Procedural Cube")
	float BoxSize = 100.0f; // 1 meter in UE units (100cm)

	// Gap between boxes
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Procedural Cube")
	float GapSize = 25.0f; // 0.25 meters in UE units (25cm)

	// Root scene component
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Components")
	class USceneComponent* SceneRoot;

private:
	// Array to hold all the static mesh components
	UPROPERTY()
	TArray<class UStaticMeshComponent*> CubeComponents;

	// Function to generate the cube
	void GenerateCube();

	// Function to clear existing cubes
	void ClearCubes();
};
