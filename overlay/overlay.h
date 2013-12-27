#ifndef O_OVERLAY_H
#define O_OVERLAY_H

typedef struct overlay overlay_t;

#include "swbuilding.h"
#include "swinventory.h"
#include "swskills.h"
#include "swequipment.h"
#include "build.h"

struct overlay
{
	swbuilding_t  swbuilding;
	swinventory_t swinventory;
	swskills_t    swskills;
	swequipment_t swequipment;
	ov_build_t    build;
};

#include "../game.h"

overlay_t* overlay_init(graphics_t* g);
void       overlay_exit(overlay_t* o);

void overlay_cursor(overlay_t* o, game_t* g);
void overlay_draw  (overlay_t* o, game_t* g);
int  overlay_catch (overlay_t* o, game_t* g, int x, int y, int t);

#endif
