#ifndef U_MINE_H
#define U_MINE_H

#include "material.h"

typedef struct kindOf_mine kindOf_mine_t;

struct kindOf_mine
{
	const char* name;

	kindOf_material_t* material;
};

#endif
