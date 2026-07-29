[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("Headphone", "Speakers")]
    [string]$Output,

    [string]$Destination
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
if ([string]::IsNullOrWhiteSpace($Destination)) {
    $Destination = Join-Path (Split-Path -Parent $PSCommandPath) "captures"
}

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

namespace Ae5Control
{
    [ComImport]
    [Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]
    internal class MMDeviceEnumeratorComObject
    {
    }

    [ComImport]
    [Guid("A95664D2-9614-4F35-A746-DE8DB63617E6")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IMMDeviceEnumerator
    {
        [PreserveSig]
        int EnumAudioEndpoints(int dataFlow, uint stateMask, out IntPtr devices);

        [PreserveSig]
        int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice endpoint);

        [PreserveSig]
        int GetDevice([MarshalAs(UnmanagedType.LPWStr)] string id, out IMMDevice endpoint);

        [PreserveSig]
        int RegisterEndpointNotificationCallback(IntPtr client);

        [PreserveSig]
        int UnregisterEndpointNotificationCallback(IntPtr client);
    }

    [ComImport]
    [Guid("D666063F-1587-4E43-81F1-B948E807363F")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IMMDevice
    {
        [PreserveSig]
        int Activate(
            ref Guid interfaceId,
            uint classContext,
            IntPtr activationParameters,
            [MarshalAs(UnmanagedType.IUnknown)] out object instance);

        [PreserveSig]
        int OpenPropertyStore(uint access, out IntPtr properties);

        [PreserveSig]
        int GetId([MarshalAs(UnmanagedType.LPWStr)] out string id);

        [PreserveSig]
        int GetState(out uint state);
    }

    [ComImport]
    [Guid("5CDF2C82-841E-4546-9722-0CF74078229A")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IAudioEndpointVolume
    {
        [PreserveSig]
        int RegisterControlChangeNotify(IntPtr notify);

        [PreserveSig]
        int UnregisterControlChangeNotify(IntPtr notify);

        [PreserveSig]
        int GetChannelCount(out uint channelCount);

        [PreserveSig]
        int SetMasterVolumeLevel(float levelDb, IntPtr eventContext);

        [PreserveSig]
        int SetMasterVolumeLevelScalar(float level, IntPtr eventContext);

        [PreserveSig]
        int GetMasterVolumeLevel(out float levelDb);

        [PreserveSig]
        int GetMasterVolumeLevelScalar(out float level);

        [PreserveSig]
        int SetChannelVolumeLevel(uint channel, float levelDb, IntPtr eventContext);

        [PreserveSig]
        int SetChannelVolumeLevelScalar(uint channel, float level, IntPtr eventContext);

        [PreserveSig]
        int GetChannelVolumeLevel(uint channel, out float levelDb);

        [PreserveSig]
        int GetChannelVolumeLevelScalar(uint channel, out float level);

        [PreserveSig]
        int SetMute([MarshalAs(UnmanagedType.Bool)] bool muted, IntPtr eventContext);

        [PreserveSig]
        int GetMute([MarshalAs(UnmanagedType.Bool)] out bool muted);

        [PreserveSig]
        int GetVolumeStepInfo(out uint currentStep, out uint stepCount);

        [PreserveSig]
        int VolumeStepUp(IntPtr eventContext);

        [PreserveSig]
        int VolumeStepDown(IntPtr eventContext);

        [PreserveSig]
        int QueryHardwareSupport(out uint hardwareSupportMask);

        [PreserveSig]
        int GetVolumeRange(out float minimumDb, out float maximumDb, out float incrementDb);
    }

    public sealed class EndpointVolumeProbe : IDisposable
    {
        private object enumeratorObject;
        private IMMDevice endpoint;
        private object volumeObject;
        private IAudioEndpointVolume volume;

        public EndpointVolumeProbe()
        {
            enumeratorObject = new MMDeviceEnumeratorComObject();
            IMMDeviceEnumerator enumerator = (IMMDeviceEnumerator)enumeratorObject;
            Check(
                enumerator.GetDefaultAudioEndpoint(0, 1, out endpoint),
                "GetDefaultAudioEndpoint(eRender, eMultimedia)");

            Guid endpointVolumeId = new Guid("5CDF2C82-841E-4546-9722-0CF74078229A");
            Check(
                endpoint.Activate(ref endpointVolumeId, 23, IntPtr.Zero, out volumeObject),
                "IMMDevice.Activate(IAudioEndpointVolume)");
            volume = (IAudioEndpointVolume)volumeObject;
        }

        public string DeviceId
        {
            get
            {
                string id;
                Check(endpoint.GetId(out id), "IMMDevice.GetId");
                return id;
            }
        }

        public bool Muted
        {
            get
            {
                bool muted;
                Check(volume.GetMute(out muted), "IAudioEndpointVolume.GetMute");
                return muted;
            }
            set
            {
                Check(
                    volume.SetMute(value, IntPtr.Zero),
                    "IAudioEndpointVolume.SetMute");
            }
        }

        public float Scalar
        {
            get
            {
                float scalar;
                Check(
                    volume.GetMasterVolumeLevelScalar(out scalar),
                    "IAudioEndpointVolume.GetMasterVolumeLevelScalar");
                return scalar;
            }
            set
            {
                Check(
                    volume.SetMasterVolumeLevelScalar(value, IntPtr.Zero),
                    "IAudioEndpointVolume.SetMasterVolumeLevelScalar");
            }
        }

        public float Decibels
        {
            get
            {
                float decibels;
                Check(
                    volume.GetMasterVolumeLevel(out decibels),
                    "IAudioEndpointVolume.GetMasterVolumeLevel");
                return decibels;
            }
        }

        public float[] GetRange()
        {
            float minimum;
            float maximum;
            float increment;
            Check(
                volume.GetVolumeRange(out minimum, out maximum, out increment),
                "IAudioEndpointVolume.GetVolumeRange");
            return new float[] { minimum, maximum, increment };
        }

        public uint[] GetStepInfo()
        {
            uint current;
            uint count;
            Check(
                volume.GetVolumeStepInfo(out current, out count),
                "IAudioEndpointVolume.GetVolumeStepInfo");
            return new uint[] { current, count };
        }

        public uint GetHardwareSupport()
        {
            uint support;
            Check(
                volume.QueryHardwareSupport(out support),
                "IAudioEndpointVolume.QueryHardwareSupport");
            return support;
        }

        public uint GetChannelCount()
        {
            uint count;
            Check(
                volume.GetChannelCount(out count),
                "IAudioEndpointVolume.GetChannelCount");
            return count;
        }

        public void Dispose()
        {
            volume = null;
            if (volumeObject != null && Marshal.IsComObject(volumeObject))
            {
                Marshal.FinalReleaseComObject(volumeObject);
            }
            if (endpoint != null && Marshal.IsComObject(endpoint))
            {
                Marshal.FinalReleaseComObject(endpoint);
            }
            if (enumeratorObject != null && Marshal.IsComObject(enumeratorObject))
            {
                Marshal.FinalReleaseComObject(enumeratorObject);
            }
            endpoint = null;
            volumeObject = null;
            enumeratorObject = null;
        }

        private static void Check(int result, string operation)
        {
            if (result < 0)
            {
                throw new COMException(operation + " failed", result);
            }
        }
    }
}
'@

function Get-EndpointName {
    param([Parameter(Mandatory = $true)][string]$DeviceId)

    try {
        $device = Get-PnpDevice -Class AudioEndpoint -PresentOnly |
            Where-Object {
                $null -ne $_.InstanceId -and $_.InstanceId.EndsWith(
                    $DeviceId,
                    [System.StringComparison]::OrdinalIgnoreCase)
            } |
            Select-Object -First 1
        if ($null -ne $device -and -not [string]::IsNullOrWhiteSpace($device.FriendlyName)) {
            return $device.FriendlyName
        }
    }
    catch {
        throw "Unable to verify the default Windows playback endpoint: $($_.Exception.Message)"
    }
    throw "The default Windows multimedia playback endpoint is not present in Plug and Play."
}

$probe = $null
$originalScalar = 0.0
$originalMute = $false
$stateCaptured = $false
$restoreVerified = $false
$capture = $null

try {
    $probe = [Ae5Control.EndpointVolumeProbe]::new()
    $endpointId = $probe.DeviceId
    $endpointName = Get-EndpointName -DeviceId $endpointId
    if (-not $endpointName.ToLowerInvariant().Contains("ae-5")) {
        throw "The default Windows multimedia endpoint is '$endpointName', not an AE-5."
    }

    $originalScalar = $probe.Scalar
    $originalMute = $probe.Muted
    $stateCaptured = $true
    $probe.Muted = $true
    if (-not $probe.Muted) {
        throw "Windows did not retain endpoint mute; no volume points were collected."
    }

    $range = $probe.GetRange()
    $steps = $probe.GetStepInfo()
    $points = [System.Collections.Generic.List[object]]::new()
    foreach ($percent in 0..100) {
        $requested = [single]($percent / 100.0)
        $probe.Scalar = $requested
        Start-Sleep -Milliseconds 5
        if (-not $probe.Muted) {
            throw "The endpoint became unmuted during collection; the capture was aborted."
        }
        $points.Add([ordered]@{
            percent = $percent
            requested_scalar = [double]$requested
            readback_scalar = [double]$probe.Scalar
            readback_db = [double]$probe.Decibels
        })
    }

    $capture = [ordered]@{
        format_version = 1
        captured_utc = [DateTime]::UtcNow.ToString("o")
        windows_version = [Environment]::OSVersion.VersionString
        endpoint_id = $endpointId
        endpoint_name = $endpointName
        role = "multimedia"
        output = $Output
        range_min_db = [double]$range[0]
        range_max_db = [double]$range[1]
        range_increment_db = [double]$range[2]
        volume_step_current = [uint32]$steps[0]
        volume_step_count = [uint32]$steps[1]
        channel_count = [uint32]$probe.GetChannelCount()
        hardware_support_mask = [uint32]$probe.GetHardwareSupport()
        restore_verified = $false
        points = $points
    }
}
finally {
    if ($null -ne $probe) {
        try {
            if ($stateCaptured) {
                $probe.Scalar = [single]$originalScalar
                $probe.Muted = $originalMute
                $restoreVerified =
                    ([Math]::Abs([double]$probe.Scalar - [double]$originalScalar) -le 0.001) -and
                    ($probe.Muted -eq $originalMute)
            }
        }
        finally {
            $probe.Dispose()
        }
    }
}

if ($null -eq $capture) {
    throw "No Windows volume-curve capture was produced."
}
if (-not $restoreVerified) {
    throw "Windows volume or mute restoration could not be verified; the capture was discarded."
}
$capture.restore_verified = $true

[System.IO.Directory]::CreateDirectory($Destination) | Out-Null
$timestamp = [DateTime]::Now.ToString("yyyyMMdd-HHmmss")
$path = Join-Path $Destination "ae5-windows-volume-$($Output.ToLowerInvariant())-$timestamp.json"
$json = $capture | ConvertTo-Json -Depth 5
$utf8 = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText($path, $json + [Environment]::NewLine, $utf8)

Write-Host "AE-5 endpoint volume curve captured without playback or OutFX writes."
Write-Host "Endpoint: $($capture.endpoint_name)"
Write-Host "Output label: $Output"
Write-Host "Points: $($capture.points.Count)"
Write-Host "Original volume and mute restored: yes"
Write-Host "Capture: $path"
