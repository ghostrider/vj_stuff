// Copyright Epic Games, Inc. All Rights Reserved.

using UnrealBuildTool;

public class VJAnimations : ModuleRules
{
	public VJAnimations(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

		PublicDependencyModuleNames.AddRange(new string[] {
			"Core",
			"CoreUObject",
			"Engine",
			"InputCore",
			"Niagara",
			"MediaAssets",
			"MediaIOCore",
			"Media",
			"AudioCapture",
			"AudioMixer",
			"AudioSynesthesia",
			"SignalProcessing"
		});

		PrivateDependencyModuleNames.AddRange(new string[] {
			"EnhancedInput",
			"RenderCore",
			"RHI"
		});

		// Uncomment if you are using Slate UI
		// PrivateDependencyModuleNames.AddRange(new string[] { "Slate", "SlateCore" });

		// Uncomment if you are using online features
		// PrivateDependencyModuleNames.Add("OnlineSubsystem");
	}
}
