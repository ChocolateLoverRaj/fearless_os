# Memory Layout
0-0x5000 - BIOS memory, must be preserved
0x5000-0x7C00 - Stack
0x7C00 - Partition Sector 0 loaded by BIOS
0x8000-0x9000 - Top level page table (points to 256 TiB)
0x9000-0xA000 - Page table (points to 512 GiB)
0xA000-0xB000 - Page table (points to 1 GiB)
0xB000 - 32-bit Rust code

# File Layout
Protective MBR
GPT

Partition:
Sector 0 (512 B)
Sector 1 (starts at sector 1, up to 604 KiB)

GPT backup

# Sector 0 stage
- Creates a minimal page table for entering long mode, identity mapping lower 1 GiB using 2 MiB mappings, even if 1 GiB mappings are supported
- Loads small stage into low mem and jumps to it

# Small stage
- Finds free phys mem to load big stage into
- Loads big stage into it
- Maps big stage at start of higher half using 1 GiB (if available) or 2 MiB mappings
- Jumps to big stage

# Big Stage Memory
## Virtual memory
Low 1 GiB: Mapped to Low 1 GiB of phys mem by early boot stages. We can just leave the mapping for now. If we have a user mode in lower mem we can unmap this (while making sure to move any needed structures). We might also need to move and repurpose this for bringing up other CPUs.

0 of higher half: This is where our kernel is mapped

1/2 of all virtual address space to end of virtual address space: Offset mapped entire physical memory, but lazily mapped. It is only mapped when we need it. It's mapped as regular memory meant to be used by our global allocator. To make things practical we will assume that we can map the entire physical memory into the kernel's address space. We will assume that there is not more than 64 TiB of physical memory. And if we wanted to support systems that did I'm sure they would support 5-level paging and would still let us map the entire physical memory into (5-level paging) virtual memory.

1/4-1/2 of virtual address space: dynamically allocated by the kernel for things like `acpi::Handler` and MMIO.

## Usable Phys Memory
- Obtained from int 0x15
- Stored as a linked list in ascending
- The actual linked list nodes are stored in global allocator tracked memory

## Used Phys Mem
- Stored as a linked list in ascending
- The actual linked list nodes are stored in global allocator tracked memory

## Global allocator
- Must ensure that all allocated phys mem is also assigned a virt addr and mapped
- Must ensure that all linked list nodes are also marked as used

## Pre-allocated used mem node memory
We need to reserve some physical memory (that's mapped) for marking other physical memory as used. We need to reserve at least 2 slots, but we can make this number more like 10,000 for performance.

## Pre-allocated page tables
To create a mapping, we might need a 512 GiB page table, and a 1 GiB page table. Most likely we won't neeed another 512 GiB table, bit just in case, we will need to pre-allocate 2 page tables that are themselves accessible through virt mem. 

## Page table memory
- Tracked by the global allocator, but remember to not drop it

## How the global allocator makes an allocation
Prerequisites (will be guaranteed to be met on init and preserved):
- List of pre-allocated used phys mem nodes len >= 0 
- Already have two empty page tables ready to be used for creating a mapping

Steps:
- Make sure that there are at least 2 pre-allocated used mem nodes (1 for this allocation, 1 for the next allocation) and make sure there are at least 4 pre-allocated page tables (2 for this mapping, 2 for the next one)
- If there aren't first, allocate more of them.
- Go through the usable phys mem linked list and the used phys mem linked list until you find the first suitable physical memory region
- Create the new node in the linked list
- Check the page tables to see if the phys mem needs to be mapped. If it isn't, map it.

## Initialization
- We will need 2 page tables ready to go for mapping the usable phys mem.
- Determine the number of usable mem nodes, which we can call N.
- Find a chunk of free phys mem that is large enough to contain (remember to consider alignment) N + 2 used mem nodes, 2 page tables.
- Make sure that chunk is mapped, using the 2 page tables we already have ready
- Create the first used phys mem node, which will mark itself and the rest of the memory we're using right now as used.
- Create usable phys mem nodes in that memory
- "Create" two page tables in that memory and mark those as the pre-allocated page tables
- Now we are ready to do allocations!
