// Copyright Epic Games, Inc. All Rights Reserved.

#include "VJCameraCapture.h"
#include "MediaPlayer.h"
#include "MediaTexture.h"
#include "MediaSource.h"
#include "UObject/ConstructorHelpers.h"

AVJCameraCapture::AVJCameraCapture()
{
	PrimaryActorTick.bCanEverTick = true;

	// Initialize media components
	MediaPlayer = CreateDefaultSubobject<UMediaPlayer>(TEXT("MediaPlayer"));
	MediaTexture = CreateDefaultSubobject<UMediaTexture>(TEXT("MediaTexture"));

	// Default settings
	CameraDeviceIndex = 0;
	bAutoStart = true;
	bIsCapturing = false;
	CaptureWidth = 1920;
	CaptureHeight = 1080;
	CaptureFrameRate = 30;
	bFlipHorizontal = false;
	bFlipVertical = false;

	// Platform-specific default camera URLs
#if PLATFORM_WINDOWS
	CameraDeviceURL = TEXT("video=");
#elif PLATFORM_LINUX
	CameraDeviceURL = TEXT("v4l2:///dev/video0");
#elif PLATFORM_MAC
	CameraDeviceURL = TEXT("avfoundation://0");
#else
	CameraDeviceURL = TEXT("");
#endif
}

void AVJCameraCapture::BeginPlay()
{
	Super::BeginPlay();

	InitializeMediaComponents();

	if (bAutoStart)
	{
		StartCapture();
	}
}

void AVJCameraCapture::EndPlay(const EEndPlayReason::Type EndPlayReason)
{
	StopCapture();
	Super::EndPlay(EndPlayReason);
}

void AVJCameraCapture::Tick(float DeltaTime)
{
	Super::Tick(DeltaTime);

	// Monitor media player status
	if (MediaPlayer && bIsCapturing)
	{
		if (!MediaPlayer->IsPlaying())
		{
			// Try to restart if playback stopped unexpectedly
			UE_LOG(LogTemp, Warning, TEXT("Camera capture stopped unexpectedly, attempting restart"));
			bIsCapturing = false;
		}
	}
}

void AVJCameraCapture::InitializeMediaComponents()
{
	if (MediaPlayer && MediaTexture)
	{
		// Link media texture to media player
		MediaTexture->SetMediaPlayer(MediaPlayer);
		MediaTexture->UpdateResource();

		// Configure media player
		MediaPlayer->PlayOnOpen = true;
		MediaPlayer->SetLooping(true);

		UE_LOG(LogTemp, Log, TEXT("VJCameraCapture: Media components initialized"));
	}
}

FString AVJCameraCapture::BuildCameraURL()
{
	FString URL;

#if PLATFORM_WINDOWS
	// Windows Media Foundation format
	// Examples: "video=USB Video Device", "video=Webcam"
	if (CameraDeviceURL.IsEmpty() || CameraDeviceURL == TEXT("video="))
	{
		// Use device index if no specific device name provided
		URL = FString::Printf(TEXT("video=%d"), CameraDeviceIndex);
	}
	else
	{
		URL = CameraDeviceURL;
	}
#elif PLATFORM_LINUX
	// V4L2 format for Linux
	// Example: "v4l2:///dev/video0"
	if (CameraDeviceURL.IsEmpty())
	{
		URL = FString::Printf(TEXT("v4l2:///dev/video%d"), CameraDeviceIndex);
	}
	else
	{
		URL = CameraDeviceURL;
	}
#elif PLATFORM_MAC
	// AVFoundation format for macOS
	// Example: "avfoundation://0"
	if (CameraDeviceURL.IsEmpty())
	{
		URL = FString::Printf(TEXT("avfoundation://%d"), CameraDeviceIndex);
	}
	else
	{
		URL = CameraDeviceURL;
	}
#else
	URL = CameraDeviceURL;
#endif

	return URL;
}

void AVJCameraCapture::StartCapture()
{
	if (!MediaPlayer)
	{
		UE_LOG(LogTemp, Error, TEXT("VJCameraCapture: MediaPlayer is null"));
		return;
	}

	if (bIsCapturing)
	{
		UE_LOG(LogTemp, Warning, TEXT("VJCameraCapture: Already capturing"));
		return;
	}

	FString URL = BuildCameraURL();

	UE_LOG(LogTemp, Log, TEXT("VJCameraCapture: Starting capture from: %s"), *URL);
	UE_LOG(LogTemp, Log, TEXT("VJCameraCapture: Resolution: %dx%d @ %d fps"), CaptureWidth, CaptureHeight, CaptureFrameRate);

	// Open the camera URL
	bool bOpened = MediaPlayer->OpenUrl(URL);

	if (bOpened)
	{
		bIsCapturing = true;
		UE_LOG(LogTemp, Log, TEXT("VJCameraCapture: Camera capture started successfully"));
	}
	else
	{
		UE_LOG(LogTemp, Error, TEXT("VJCameraCapture: Failed to open camera URL: %s"), *URL);
		UE_LOG(LogTemp, Error, TEXT("VJCameraCapture: Make sure the camera device exists and is not in use by another application"));
	}
}

void AVJCameraCapture::StopCapture()
{
	if (!MediaPlayer)
	{
		return;
	}

	if (!bIsCapturing)
	{
		return;
	}

	MediaPlayer->Close();
	bIsCapturing = false;

	UE_LOG(LogTemp, Log, TEXT("VJCameraCapture: Camera capture stopped"));
}

void AVJCameraCapture::RestartCapture()
{
	StopCapture();

	// Small delay before restart
	FTimerHandle TimerHandle;
	GetWorld()->GetTimerManager().SetTimer(TimerHandle, [this]()
	{
		StartCapture();
	}, 0.5f, false);
}

void AVJCameraCapture::SetCameraDevice(int32 DeviceIndex)
{
	bool bWasCapturing = bIsCapturing;

	if (bWasCapturing)
	{
		StopCapture();
	}

	CameraDeviceIndex = DeviceIndex;

	if (bWasCapturing)
	{
		StartCapture();
	}

	UE_LOG(LogTemp, Log, TEXT("VJCameraCapture: Camera device set to index %d"), DeviceIndex);
}
