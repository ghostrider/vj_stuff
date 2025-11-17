// Copyright Epic Games, Inc. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "MediaPlayer.h"
#include "MediaTexture.h"
#include "MediaSource.h"
#include "VJCameraCapture.generated.h"

/**
 * VJ Camera Capture - Handles USB camera/video capture device input
 * Captures video from webcams, capture cards, or other video input devices
 * Outputs to a Media Texture that can be used in materials
 */
UCLASS()
class VJANIMATIONS_API AVJCameraCapture : public AActor
{
	GENERATED_BODY()

public:
	AVJCameraCapture();

protected:
	virtual void BeginPlay() override;
	virtual void EndPlay(const EEndPlayReason::Type EndPlayReason) override;

public:
	virtual void Tick(float DeltaTime) override;

	/** Media Player for camera capture */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "VJ Camera")
	UMediaPlayer* MediaPlayer;

	/** Media Texture output (use this in materials) */
	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "VJ Camera")
	UMediaTexture* MediaTexture;

	/** Camera device URL (e.g., "v4l2:///dev/video0" on Linux, "video=USB Video Device" on Windows) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VJ Camera|Settings")
	FString CameraDeviceURL;

	/** Camera device index (0 = first camera, 1 = second, etc.) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VJ Camera|Settings", meta = (ClampMin = "0", ClampMax = "10"))
	int32 CameraDeviceIndex;

	/** Auto-start camera on begin play */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VJ Camera|Settings")
	bool bAutoStart;

	/** Enable camera capture */
	UPROPERTY(BlueprintReadOnly, Category = "VJ Camera|Status")
	bool bIsCapturing;

	/** Desired camera resolution width */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VJ Camera|Settings")
	int32 CaptureWidth;

	/** Desired camera resolution height */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VJ Camera|Settings")
	int32 CaptureHeight;

	/** Capture frame rate */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VJ Camera|Settings")
	int32 CaptureFrameRate;

	/** Flip camera horizontally (mirror) */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VJ Camera|Settings")
	bool bFlipHorizontal;

	/** Flip camera vertically */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "VJ Camera|Settings")
	bool bFlipVertical;

	/** Start capturing from camera */
	UFUNCTION(BlueprintCallable, Category = "VJ Camera")
	void StartCapture();

	/** Stop capturing from camera */
	UFUNCTION(BlueprintCallable, Category = "VJ Camera")
	void StopCapture();

	/** Restart capture with current settings */
	UFUNCTION(BlueprintCallable, Category = "VJ Camera")
	void RestartCapture();

	/** Set camera device by index */
	UFUNCTION(BlueprintCallable, Category = "VJ Camera")
	void SetCameraDevice(int32 DeviceIndex);

	/** Get the media texture for use in materials */
	UFUNCTION(BlueprintCallable, Category = "VJ Camera")
	UMediaTexture* GetMediaTexture() const { return MediaTexture; }

	/** Check if camera is currently capturing */
	UFUNCTION(BlueprintCallable, Category = "VJ Camera")
	bool IsCapturing() const { return bIsCapturing; }

private:
	void InitializeMediaComponents();
	FString BuildCameraURL();
};
