using System;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Threading.Tasks;
using System.Windows.Forms;

internal static class Bootstrap
{
    [STAThread]
    private static int Main(string[] args)
    {
        if (Array.Exists(args, argument => argument == "--verify")) return VerifyPayload();
        string temp = Path.Combine(Path.GetTempPath(), "HushMic-Setup-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(temp);
            string payload = Path.Combine(temp, "HushMicPayload.zip");
            string installer = Path.Combine(temp, "Install-HushMic.ps1");
            Extract("HushMicPayload.zip", payload);
            Extract("Install-HushMic.ps1", installer);

            string powershell = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Windows),
                "System32", "WindowsPowerShell", "v1.0", "powershell.exe");
            ProcessStartInfo start = new ProcessStartInfo(powershell)
            {
                Arguments = "-NoProfile -ExecutionPolicy Bypass -File \"" + installer + "\" -PayloadZip \"" + payload + "\"",
                WorkingDirectory = temp,
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true
            };
            using (Process process = Process.Start(start))
            {
                Task<string> stdout = process.StandardOutput.ReadToEndAsync();
                Task<string> stderr = process.StandardError.ReadToEndAsync();
                process.WaitForExit();
                string details = (stderr.Result + Environment.NewLine + stdout.Result).Trim();
                if (process.ExitCode != 0)
                {
                    MessageBox.Show("A instalação do Crunchy AI falhou (código " + process.ExitCode + ").\n\n" + details,
                        "Instalador Crunchy AI", MessageBoxButtons.OK, MessageBoxIcon.Error);
                    return process.ExitCode;
                }
            }
            string app = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "Programs", "HushMic", "HushMic.exe");
            if (File.Exists(app)) Process.Start(new ProcessStartInfo(app) { UseShellExecute = true });
            MessageBox.Show("Crunchy AI instalado para este usuário.\n\nNo Discord ou OBS, selecione CABLE Output (VB-Audio Virtual Cable) como microfone. O VB-CABLE é instalado separadamente.",
                "Crunchy AI", MessageBoxButtons.OK, MessageBoxIcon.Information);
            return 0;
        }
        catch (Exception error)
        {
            MessageBox.Show("Não foi possível iniciar a instalação do Crunchy AI.\n\n" + error.Message,
                "Instalador Crunchy AI", MessageBoxButtons.OK, MessageBoxIcon.Error);
            return 1;
        }
        finally
        {
            try { if (Directory.Exists(temp)) Directory.Delete(temp, true); }
            catch { /* Keep the system temporary folder for Windows cleanup. */ }
        }
    }

    private static int VerifyPayload()
    {
        try
        {
            using (Stream script = Assembly.GetExecutingAssembly().GetManifestResourceStream("Install-HushMic.ps1"))
                if (script == null) throw new InvalidDataException("Script de instalação incorporado ausente.");
            using (Stream stream = Assembly.GetExecutingAssembly().GetManifestResourceStream("HushMicPayload.zip"))
            {
                if (stream == null) throw new InvalidDataException("Pacote incorporado ausente.");
                using (ZipArchive archive = new ZipArchive(stream, ZipArchiveMode.Read))
                {
                    string[] required = { "HushMic.exe", "install-local.ps1", "uninstall-local.ps1",
                        "LICENSE-APACHE", "LICENSE-MIT", "THIRD_PARTY_NOTICES.md",
                        "assets/crunchy-ai-icon.png", "assets/crunchy-ai.ico",
                        "assets/dpdfnet2_48khz_hr.onnx", "assets/dpdfnet8_48khz_hr.onnx",
                        "assets/ONNX-Runtime-LICENSE.txt", "assets/onnxruntime_providers_shared.dll",
                        "assets/onnxruntime.dll", "assets/ThirdPartyNotices.txt" };
                    foreach (string name in required)
                        if (archive.GetEntry(name) == null) throw new InvalidDataException("Arquivo ausente: " + name);
                    foreach (ZipArchiveEntry entry in archive.Entries)
                    {
                        string name = entry.FullName;
                        if (name.IndexOf("vbcable/", StringComparison.OrdinalIgnoreCase) >= 0 ||
                            name.EndsWith(".sys", StringComparison.OrdinalIgnoreCase) ||
                            name.EndsWith(".cat", StringComparison.OrdinalIgnoreCase) ||
                            name.EndsWith(".inf", StringComparison.OrdinalIgnoreCase) ||
                            name.EndsWith(".cer", StringComparison.OrdinalIgnoreCase))
                            throw new InvalidDataException("O pacote inclui arquivo de driver: " + name);
                    }
                }
            }
            return 0;
        }
        catch (Exception error)
        {
            File.WriteAllText(Path.Combine(Path.GetTempPath(), "HushMic-Setup-verify-error.txt"), error.ToString());
            return 1;
        }
    }

    private static void Extract(string resourceName, string target)
    {
        using (Stream source = Assembly.GetExecutingAssembly().GetManifestResourceStream(resourceName))
        {
            if (source == null) throw new InvalidDataException("Recurso ausente: " + resourceName);
            using (FileStream destination = File.Create(target)) source.CopyTo(destination);
        }
    }
}
