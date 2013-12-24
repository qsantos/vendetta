#ifndef W_BUILDING_H
#define W_BUILDING_H

typedef struct building building_t;

#include "object.h"
#include "../universe/building.h"

struct building
{
	object_t o;

	kindOf_building_t* t;

	float buildProgress;
};

void building_init(building_t* b, kindOf_building_t* t, float x, float y);
void building_exit(building_t* b);

#endif
