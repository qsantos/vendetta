#include "material.h"

#include <stdlib.h>

void kindOf_material_exit(kindOf_material_t* m)
{
	free(m->name);
}
