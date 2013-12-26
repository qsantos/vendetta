#include "equipment.h"

#include <stdlib.h>

void kindOf_slot_init(kindOf_slot_t* s)
{
	s->name = NULL;
}


void kindOf_slot_exit(kindOf_slot_t* s)
{
	free(s->name);
}
