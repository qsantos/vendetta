#ifndef U_MINE_H
#define U_MINE_H

#include "material.h"

typedef struct kindOf_mine kindOf_mine_t;

struct kindOf_mine
{
	int id;
	const char* name;

	int material_id;
	kindOf_material_t* material;
};

#endif
