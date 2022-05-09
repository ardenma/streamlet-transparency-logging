import time
import subprocess
import os

from threading import Thread

from mininet.topo import Topo
from mininet.net import Mininet
from mininet.cli import CLI
from mininet.util import dumpNodeConnections
from mininet.log import setLogLevel

N=4
PROG="p2p_network_test/target/debug/p2p_network_test"

# Build on the Topo base class
# We're not doing anything with switches, so we can have 
# a bunch of nodes linked together by one switch. 
class MultiNodeTopo(Topo):

    # Override the default topology 
    def build(self, n=10, configFile = ""):
        if configFile != "": 
            print("we could configure stuff here")
        # Initialize switch
        # Returns Switch object
        switch = self.addSwitch('s1')
        # Add `n` hosts, all linked to the switch
        for h in range(n): 
            # Run on local network (default that we're given is 10.0.0.0/8 to work with; 
            # i.e., these hosts can communicate with anyone else in the same 10.0.0.0/8 subnet.)
            # Each host gets exactly one IP address, ranging from 10.0.0.1 -> 10.0.0.(n+1)
            host = self.addHost('h' + str(h + 1), ip = '10.0.0.' + str(h + 1))
            
            # Note: We can configure parameters on these links, 
            # e.g. different bandwidth, delay, and packet loss. 
            self.addLink(host, switch)

# VERY temporary way of setting up everyone to send us output
# There is... probably a better way to do this? 
class HostRunner(): 
    def __init__(self, net, netsize, program_str): 
        self.program_str = program_str
        self.hosts = [net.get("h" + str(i + 1)) for i in range(netsize)]

    def run(self):
        for i in range(len(self.hosts)):
            pipe = self.hosts[i].popen([self.program_str, 'h' + str(i + 1)], stdout=subprocess.PIPE, bufsize=1, universal_newlines=True)
            thread = Thread(target=self.print_lines, args=[pipe])
            thread.start()

    def print_lines(self, p): 
        for line in p.stdout:
            print(line)

# Clean up between runs:
os.system("sudo mn -c")

# Set up the Net
topo = MultiNodeTopo(N)
net = Mininet(topo)
net.start()

runner = HostRunner(net, N, PROG)
runner.run()

# Indefinitely sleep for the demo?
time.sleep(10)

net.stop()
