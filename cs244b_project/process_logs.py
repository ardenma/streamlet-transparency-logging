import os

import matplotlib
import numpy as np
import pandas as pd
import seaborn as sns
import matplotlib.pyplot as plt

NORMAL_LOGDIR = "./normal_logs"
COMPROMISED_LOGDIR = "./compromised_logs"
PLOTDIR = "./plots"


matplotlib.rcParams.update({'font.size': 20, 'axes.titlepad': 20})
matplotlib.rc('xtick', labelsize=16) 
matplotlib.rc('ytick', labelsize=16) 


os.makedirs(PLOTDIR, exist_ok=True)

############################################### Normal Behavior #######################################################

all_df_list = []
for dirname in os.scandir(NORMAL_LOGDIR):
    if dirname.is_dir():
        num_nodes = os.path.basename(dirname)[0]
        df_list = []
        
        # Loop through all files in a directory
        for filename in os.scandir(dirname):
            df = pd.read_csv(filename, index_col=None, header=None)
            df_list.append(df)
        
        frame = pd.concat(df_list, axis=0, ignore_index=True)
        frame.columns = ["num_nodes", "epoch_length", "data_type", "num_finalized", "finalization_percentage"]
        frame = frame.groupby(["num_nodes", "epoch_length", "data_type"]).mean().reset_index()
        all_df_list.append(frame)

df = pd.concat(all_df_list, axis=0, ignore_index=True)
df["epoch_finalization_percentage"] = df["num_finalized"] / 19 * 100
print(df.to_string())

############################################### Percentage of Epoch Finalizations #######################################################
# fig, axes = plt.subplots(1, 3, figsize=(16, 4))
# fig.tight_layout(pad=4)
# plt.suptitle("% of Epochs with Finalized Block")

figsize = (10,8)

plt.figure(figsize=figsize)

df_epoch_1_data_small = df[(df["epoch_length"] == 1) & (df["data_type"] == "small")]
df_epoch_5_data_small = df[(df["epoch_length"] == 5) & (df["data_type"] == "small")]
df_epoch_10_data_small = df[(df["epoch_length"] == 10) & (df["data_type"] == "small")]

sns.lineplot(x=df_epoch_1_data_small["num_nodes"], y=df_epoch_1_data_small["epoch_finalization_percentage"]);
sns.lineplot(x=df_epoch_5_data_small["num_nodes"], y=df_epoch_5_data_small["epoch_finalization_percentage"]);
sns.lineplot(x=df_epoch_10_data_small["num_nodes"], y=df_epoch_10_data_small["epoch_finalization_percentage"]);

plt.title("% of Epochs with Finalize Block (1B Block)")
plt.xlabel("Number of Streamlet Instances")
plt.ylabel("% of Epochs with Finalized Block")
plt.legend(labels=["1s Epoch", "5s Epoch", "10s Epoch"])
plt.savefig(os.path.join(PLOTDIR, "1b.png"))

plt.figure(figsize=figsize)

df_epoch_1_data_medium = df[(df["epoch_length"] == 1) & (df["data_type"] == "medium")]
df_epoch_5_data_medium = df[(df["epoch_length"] == 5) & (df["data_type"] == "medium")]
df_epoch_10_data_medium = df[(df["epoch_length"] == 10) & (df["data_type"] == "medium")]

sns.lineplot(x=df_epoch_1_data_medium["num_nodes"], y=df_epoch_1_data_medium["epoch_finalization_percentage"]);
sns.lineplot(x=df_epoch_5_data_medium["num_nodes"], y=df_epoch_5_data_medium["epoch_finalization_percentage"]);
sns.lineplot(x=df_epoch_10_data_medium["num_nodes"], y=df_epoch_10_data_medium["epoch_finalization_percentage"]);

plt.title("% of Epochs with Finalize Block (1KB Block)")
plt.xlabel("Number of Streamlet Instances")
plt.ylabel("% of Epochs with Finalized Block")
plt.legend(labels=["1s Epoch", "5s Epoch", "10s Epoch"])
plt.savefig(os.path.join(PLOTDIR, "1kb.png"))

plt.figure(figsize=figsize)

df_epoch_1_data_large = df[(df["epoch_length"] == 1) & (df["data_type"] == "large")]
df_epoch_5_data_large = df[(df["epoch_length"] == 5) & (df["data_type"] == "large")]
df_epoch_10_data_large = df[(df["epoch_length"] == 10) & (df["data_type"] == "large")]

sns.lineplot(x=df_epoch_1_data_large["num_nodes"], y=df_epoch_1_data_large["epoch_finalization_percentage"]);
sns.lineplot(x=df_epoch_5_data_large["num_nodes"], y=df_epoch_5_data_large["epoch_finalization_percentage"]);
sns.lineplot(x=df_epoch_10_data_large["num_nodes"], y=df_epoch_10_data_large["epoch_finalization_percentage"]);

plt.title("% of Epochs with Finalize Block (10KB Block)")
plt.xlabel("Number of Streamlet Instances")
plt.ylabel("% of Epochs with Finalized Block")
plt.legend(labels=["1s Epoch", "5s Epoch", "10s Epoch"])
plt.savefig(os.path.join(PLOTDIR, "10kb.png"))

############################################### Compromised Behavior #######################################################
NUM_FLAGS = 6
NUM_TRIALS = 1
COMPROMISE_TYPES = ["early_epoch", "late_epoch", "no_propose", "wrong_parent_hash", "no_vote", "non_leader_propose"]

all_df_list = []
for dirname in os.scandir(COMPROMISED_LOGDIR):
    if dirname.is_dir():
        num_nodes = int(os.path.basename(dirname)[0])
        results_list = []
        
        # Loop through all files in a directory
        for filename in os.scandir(dirname):
            if os.path.basename(filename) != f"Test{num_nodes -1}.log":  # compromised node
                results = [0, 0, 0, 0, 0, 0]
                df = pd.read_csv(filename, index_col=None, header=None)
                df.columns = ["num_nodes", "epoch_length", "data_type", "num_finalized", "finalization_percentage", "compromised_flags"]
                for i, n in enumerate(df["num_finalized"]):
                    results[i % 6] += n
                results = list(map(lambda x: x / NUM_TRIALS, results))
                results_list.append(results)

        results = np.mean(results_list, axis=0) / 19 * 100
        print(results)

    plt.figure(figsize=(12,6))
    sns.barplot(x=COMPROMISE_TYPES, y=results)
    plt.title(f"Streamlet Finalization Performance with {num_nodes-1} Honest Nodes and 1 Malicious Node")
    plt.xlabel("Compromise Type")
    plt.ylabel("% of Epochs with Finalized Block")
    plt.savefig(os.path.join(PLOTDIR, f"{num_nodes}_nodes_compromise_types.png"))

#######################################################################################################################################