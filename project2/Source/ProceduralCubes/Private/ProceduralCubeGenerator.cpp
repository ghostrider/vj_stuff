// Copyright Epic Games, Inc. All Rights Reserved.

#include "ProceduralCubeGenerator.h"
#include "Components/StaticMeshComponent.h"
#include "UObject/ConstructorHelpers.h"
#include "Engine/StaticMesh.h"

// Sets default values
AProceduralCubeGenerator::AProceduralCubeGenerator()
{
	// Set this actor to call Tick() every frame. Not needed for this simple example
	PrimaryActorTick.bCanEverTick = false;

	// Create the root scene component
	SceneRoot = CreateDefaultSubobject<USceneComponent>(TEXT("SceneRoot"));
	RootComponent = SceneRoot;
}

void AProceduralCubeGenerator::OnConstruction(const FTransform& Transform)
{
	Super::OnConstruction(Transform);

	// Generate the cube whenever the actor is reconstructed
	GenerateCube();
}

void AProceduralCubeGenerator::BeginPlay()
{
	Super::BeginPlay();
}

void AProceduralCubeGenerator::ClearCubes()
{
	// Destroy all existing cube components
	for (UStaticMeshComponent* Component : CubeComponents)
	{
		if (Component)
		{
			Component->DestroyComponent();
		}
	}
	CubeComponents.Empty();
}

void AProceduralCubeGenerator::GenerateCube()
{
	// Clear any existing cubes first
	ClearCubes();

	// Load the default cube mesh from engine content
	static ConstructorHelpers::FObjectFinder<UStaticMesh> CubeMeshAsset(TEXT("/Engine/BasicShapes/Cube"));
	UStaticMesh* CubeMesh = nullptr;

	if (CubeMeshAsset.Succeeded())
	{
		CubeMesh = CubeMeshAsset.Object;
	}
	else
	{
		// If we can't find the cube mesh, log an error and return
		UE_LOG(LogTemp, Error, TEXT("ProceduralCubeGenerator: Could not find cube mesh"));
		return;
	}

	// Calculate the spacing between box centers (box size + gap)
	float Spacing = BoxSize + GapSize;

	// Calculate the offset to center the entire cube structure
	float TotalSize = (CubeSize - 1) * Spacing;
	float Offset = TotalSize / 2.0f;

	// Generate the 10x10x10 cube
	for (int32 X = 0; X < CubeSize; X++)
	{
		for (int32 Y = 0; Y < CubeSize; Y++)
		{
			for (int32 Z = 0; Z < CubeSize; Z++)
			{
				// Create a unique name for this component
				FString ComponentName = FString::Printf(TEXT("Cube_%d_%d_%d"), X, Y, Z);

				// Create a new static mesh component
				UStaticMeshComponent* CubeComponent = NewObject<UStaticMeshComponent>(this, FName(*ComponentName));

				if (CubeComponent)
				{
					// Set the static mesh
					CubeComponent->SetStaticMesh(CubeMesh);

					// Register the component
					CubeComponent->RegisterComponent();

					// Attach to root
					CubeComponent->AttachToComponent(RootComponent, FAttachmentTransformRules::KeepRelativeTransform);

					// Calculate position (centered around origin)
					FVector Position;
					Position.X = (X * Spacing) - Offset;
					Position.Y = (Y * Spacing) - Offset;
					Position.Z = (Z * Spacing) - Offset;

					// Set the relative location
					CubeComponent->SetRelativeLocation(Position);

					// Set the scale to match the box size (default cube is 100x100x100, so scale to BoxSize)
					float Scale = BoxSize / 100.0f;
					CubeComponent->SetRelativeScale3D(FVector(Scale, Scale, Scale));

					// Enable collision
					CubeComponent->SetCollisionEnabled(ECollisionEnabled::QueryAndPhysics);

					// Add to our array of components
					CubeComponents.Add(CubeComponent);
				}
			}
		}
	}

	UE_LOG(LogTemp, Log, TEXT("ProceduralCubeGenerator: Generated %d cubes in a %dx%dx%d arrangement"),
		CubeComponents.Num(), CubeSize, CubeSize, CubeSize);
}
