import os

import pandas as pd
import seaborn as sns
import matplotlib.pyplot as plt

LOGDIR = "./normal_logs"
PLOTDIR = "./plots"


os.makedirs(PLOTDIR, exist_ok=True)

all_df_list = []

for dirname in os.scandir(LOGDIR):
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
fig, axes = plt.subplots(1, 3, figsize=(16, 4))
fig.tight_layout(pad=4)
plt.suptitle("% of Epochs with Finalized Block")

df_epoch_1_data_small = df[(df["epoch_length"] == 1) & (df["data_type"] == "small")]
df_epoch_5_data_small = df[(df["epoch_length"] == 5) & (df["data_type"] == "small")]
df_epoch_10_data_small = df[(df["epoch_length"] == 10) & (df["data_type"] == "small")]

sns.lineplot(ax=axes[0], x=df_epoch_1_data_small["num_nodes"], y=df_epoch_1_data_small["epoch_finalization_percentage"]);
sns.lineplot(ax=axes[0], x=df_epoch_5_data_small["num_nodes"], y=df_epoch_5_data_small["epoch_finalization_percentage"]);
sns.lineplot(ax=axes[0], x=df_epoch_10_data_small["num_nodes"], y=df_epoch_10_data_small["epoch_finalization_percentage"]);

axes[0].set_title("1B Block")
axes[0].set_xlabel("Number of Streamlet Instances")
axes[0].set_ylabel("% of Epochs with Finalized Block")
axes[0].legend(labels=["1s Epoch", "5s Epoch", "10s Epoch"])

df_epoch_1_data_medium = df[(df["epoch_length"] == 1) & (df["data_type"] == "medium")]
df_epoch_5_data_medium = df[(df["epoch_length"] == 5) & (df["data_type"] == "medium")]
df_epoch_10_data_medium = df[(df["epoch_length"] == 10) & (df["data_type"] == "medium")]

sns.lineplot(ax=axes[1], x=df_epoch_1_data_medium["num_nodes"], y=df_epoch_1_data_medium["epoch_finalization_percentage"]);
sns.lineplot(ax=axes[1], x=df_epoch_5_data_medium["num_nodes"], y=df_epoch_5_data_medium["epoch_finalization_percentage"]);
sns.lineplot(ax=axes[1], x=df_epoch_10_data_medium["num_nodes"], y=df_epoch_10_data_medium["epoch_finalization_percentage"]);

axes[1].set_title("1KB Block")
axes[1].set_xlabel("Number of Streamlet Instances")
axes[1].set_ylabel("% of Epochs with Finalized Block")
axes[1].legend(labels=["1s Epoch", "5s Epoch", "10s Epoch"])

df_epoch_1_data_large = df[(df["epoch_length"] == 1) & (df["data_type"] == "large")]
df_epoch_5_data_large = df[(df["epoch_length"] == 5) & (df["data_type"] == "large")]
df_epoch_10_data_large = df[(df["epoch_length"] == 10) & (df["data_type"] == "large")]

sns.lineplot(ax=axes[2], x=df_epoch_1_data_large["num_nodes"], y=df_epoch_1_data_large["epoch_finalization_percentage"]);
sns.lineplot(ax=axes[2], x=df_epoch_5_data_large["num_nodes"], y=df_epoch_5_data_large["epoch_finalization_percentage"]);
sns.lineplot(ax=axes[2], x=df_epoch_10_data_large["num_nodes"], y=df_epoch_10_data_large["epoch_finalization_percentage"]);

axes[2].set_title("10KB Block")
axes[2].set_xlabel("Number of Streamlet Instances")
axes[2].set_ylabel("% of Epochs with Finalized Block")
axes[2].legend(labels=["1s Epoch", "5s Epoch", "10s Epoch"])

plt.savefig(os.path.join(PLOTDIR, "percentage_of_epochs_with_finalizations.png"))
#######################################################################################################################################