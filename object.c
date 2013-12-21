#include "object.h"

#include <math.h>

#include "character.h"
#include "mine.h"

int object_isAt(object_t* o, float x, float y)
{
	return fabs(x - o->x) <= o->w/2 && o->y - o->h <= y && y <= o->y;
}
